//! 解析参数、准备主配置和日志路径。

use std::{
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

use nasnet::{
    config::AppConfig,
    store::{StoreConnection, StoreError},
};

use super::AppError;

pub struct Startup {
    pub config: AppConfig,
    pub store: StoreConnection,
    pub log_path: PathBuf,
    pub selected_id: Option<String>,
}

/// 参数通过校验后才访问文件系统。
pub fn prepare(arguments: impl IntoIterator<Item = OsString>) -> Result<Startup, AppError> {
    let (config_argument, selected_id) = parse_arguments(arguments)?;
    let store = match config_argument {
        // 命令行路径以当前工作目录为基准。
        Some(path) => StoreConnection::from_config_path(std::path::absolute(path)?),
        None => StoreConnection::new(
            std::env::current_exe()?
                .parent()
                .ok_or(StoreError::InvalidConfigPath)?,
        ),
    };
    let config_path = store.config_path();
    let config_directory = match (config_path.parent(), config_path.file_name()) {
        (Some(directory), Some(_)) => directory,
        _ => return Err(StoreError::InvalidConfigPath.into()),
    };
    let config = store.load_app(false)?;
    let log_path = config_directory.join(&config.logging.file_path);
    reject_log_path_conflicts(&log_path, &config, config_path, config_directory)?;

    Ok(Startup {
        config,
        store,
        log_path,
        selected_id,
    })
}

fn parse_arguments(
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<(Option<PathBuf>, Option<String>), AppError> {
    let mut arguments = arguments.into_iter();
    let mut config_path = None;
    let mut selected_id = None;
    while let Some(argument) = arguments.next() {
        if argument == "--config" && config_path.is_none() {
            let value = option_value(&mut arguments).ok_or(AppError::InvalidArguments(
                "--config requires a configuration file path",
            ))?;
            config_path = Some(PathBuf::from(value));
        } else if argument == "--device" && selected_id.is_none() {
            let id = option_value(&mut arguments)
                .ok_or(AppError::InvalidArguments("--device requires a device ID"))?
                .into_string()
                .map_err(|_| AppError::InvalidArguments("device ID must contain valid Unicode"))?;
            selected_id = Some(id);
        } else {
            return Err(AppError::InvalidArguments(
                "use [--config <file>] [--device <id>]; options must not be repeated",
            ));
        }
    }
    Ok((config_path, selected_id))
}

fn option_value(arguments: &mut impl Iterator<Item = OsString>) -> Option<OsString> {
    arguments.next().filter(|value| {
        let text = value.to_string_lossy();
        !text.trim().is_empty() && !text.starts_with("--")
    })
}

/// 避免把日志写进主配置或设备文件。
fn reject_log_path_conflicts(
    log_path: &Path,
    config: &AppConfig,
    config_path: &Path,
    config_directory: &Path,
) -> Result<(), AppError> {
    let log_path = comparison_path(log_path);
    if log_path == comparison_path(config_path)
        || config
            .devices
            .iter()
            .any(|reference| log_path == comparison_path(&config_directory.join(&reference.path)))
    {
        return Err(AppError::Logging(
            "log file path conflicts with a configuration file".to_owned(),
        ));
    }
    Ok(())
}

fn comparison_path(path: &Path) -> PathBuf {
    let mut lexical = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                lexical.pop();
            }
            component => lexical.push(component.as_os_str()),
        }
    }
    // 优先保留符号链接的实际含义。
    let mut resolved = path
        .canonicalize()
        .or_else(|_| lexical.canonicalize())
        .unwrap_or(lexical);
    if cfg!(windows) {
        resolved.as_mut_os_string().make_ascii_lowercase();
    }
    resolved
}
