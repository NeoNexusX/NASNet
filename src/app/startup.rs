//! 先解析参数和读取主配置，再由 main 安装日志；此阶段不发 tracing 事件。

use std::{
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

use nasnet::{
    config::AppConfig,
    error::{AppError, StoreError},
    store::StoreConnection,
};

pub struct Startup {
    pub selected_id: Option<String>,
    pub config: AppConfig,
    pub config_directory: PathBuf,
    pub store: StoreConnection,
}

pub fn prepare(arguments: Vec<OsString>) -> Result<Startup, AppError> {
    let mut arguments = arguments.into_iter();
    let mut config_argument = None;
    let mut selected_id = None;
    while let Some(argument) = arguments.next() {
        if argument == "--config" && config_argument.is_none() {
            let value = arguments.next().ok_or(AppError::InvalidArguments(
                "--config requires a configuration file path",
            ))?;
            if value.is_empty() || value.to_str().is_some_and(|value| value.starts_with("--")) {
                return Err(AppError::InvalidArguments(
                    "--config requires a configuration file path",
                ));
            }
            config_argument = Some(PathBuf::from(value));
        } else if argument == "--device" && selected_id.is_none() {
            let value = arguments
                .next()
                .ok_or(AppError::InvalidArguments("--device requires a device ID"))?;
            let id = value.to_str().ok_or(AppError::InvalidArguments(
                "device ID must contain valid Unicode",
            ))?;
            if id.trim().is_empty() || id.starts_with("--") {
                return Err(AppError::InvalidArguments("--device requires a device ID"));
            }
            selected_id = Some(id.to_owned());
        } else {
            return Err(AppError::InvalidArguments(
                "use [--config <file>] [--device <id>]; options must not be repeated",
            ));
        }
    }

    let explicit_config = config_argument.is_some();
    let config_path = if let Some(path) = config_argument {
        // 命令行相对路径只在这里相对 cwd 转为绝对路径；内部路径统一基于配置目录。
        std::path::absolute(path)?
    } else {
        let executable = std::env::current_exe()?;
        executable
            .parent()
            .ok_or(StoreError::InvalidConfigPath)?
            .join("config.toml")
    };
    if config_path.file_name().is_none() {
        return Err(StoreError::InvalidConfigPath.into());
    }
    let config_directory = config_path
        .parent()
        .ok_or(StoreError::InvalidConfigPath)?
        .to_path_buf();
    let store = StoreConnection::from_config_path(config_path);
    let config = if explicit_config {
        store.load_app_required()?
    } else {
        store.load_app()?
    };
    reject_log_path_conflicts(&config, store.config_path(), &config_directory)?;

    Ok(Startup {
        selected_id,
        config,
        config_directory,
        store,
    })
}

/// 打开日志前保护主文件和已登记设备文件；只比较路径，不读取设备正文或创建文件。
fn reject_log_path_conflicts(
    config: &AppConfig,
    config_path: &Path,
    config_directory: &Path,
) -> Result<(), AppError> {
    let log_path = if config.logging.file_path.is_absolute() {
        config.logging.file_path.clone()
    } else {
        config_directory.join(&config.logging.file_path)
    };
    let log_path = comparison_path(&log_path);
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

/// 已存在路径优先解析符号链接；其余路径消去 . / ..，Windows 再做基本 ASCII 大小写比较。
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
    // 原路径解析优先，保留已存在符号链接的实际含义；否则尝试规整后的已存在路径。
    let mut resolved = path
        .canonicalize()
        .or_else(|_| lexical.canonicalize())
        .unwrap_or(lexical);
    if cfg!(windows) {
        resolved.as_mut_os_string().make_ascii_lowercase();
    }
    resolved
}
