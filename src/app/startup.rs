//! 启动准备：解析命令行参数，定位并读取主配置，确定日志路径。只读文件，不安装日志，也不发事件。

use std::{
    ffi::OsString,
    path::{Component, Path, PathBuf},
};

use nasnet::{
    config::AppConfig,
    store::{StoreConnection, StoreError},
};

use super::AppError;

/// 后续步骤需要的一切：已校验的主配置、存储入口、日志文件路径和选中的设备。
pub struct Startup {
    pub config: AppConfig,
    pub store: StoreConnection,
    pub log_path: PathBuf,
    pub selected_id: Option<String>,
}

/// 先解析参数，参数有误时不访问任何文件；再读取主配置并检查日志路径。
pub fn prepare(arguments: impl IntoIterator<Item = OsString>) -> Result<Startup, AppError> {
    let (config_argument, selected_id) = parse_arguments(arguments)?;
    let explicit_config = config_argument.is_some();
    let config_path = match config_argument {
        // 命令行的相对路径相对启动时的 cwd；之后的内部路径都以主配置所在目录为基准。
        Some(path) => std::path::absolute(path)?,
        None => std::env::current_exe()?
            .parent()
            .ok_or(StoreError::InvalidConfigPath)?
            .join("config.toml"),
    };
    let config_directory = match (config_path.parent(), config_path.file_name()) {
        (Some(directory), Some(_)) => directory.to_path_buf(),
        _ => return Err(StoreError::InvalidConfigPath.into()),
    };
    let store = StoreConnection::from_config_path(config_path);
    let config = if explicit_config {
        store.load_app_required()?
    } else {
        store.load_app()?
    };
    // 相对路径接在配置目录后面；绝对路径 join 后会整体替换配置目录。
    let log_path = config_directory.join(&config.logging.file_path);
    reject_log_path_conflicts(&log_path, &config, store.config_path(), &config_directory)?;

    Ok(Startup {
        config,
        store,
        log_path,
        selected_id,
    })
}

/// 接受 [--config <文件>] [--device <ID>]，顺序任意，每个选项最多一次。
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

/// 取出选项后面的值；缺失、只有空白或以 -- 开头（像另一个选项）时视为没有提供。
fn option_value(arguments: &mut impl Iterator<Item = OsString>) -> Option<OsString> {
    arguments.next().filter(|value| {
        let text = value.to_string_lossy();
        !text.trim().is_empty() && !text.starts_with("--")
    })
}

/// 打开日志前保护主文件和已登记设备文件；只比较路径，不读取设备正文或创建文件。
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
