//! 应用边界的双输出日志与错误报告；模型和存储只返回 Result。

use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    sync::Mutex,
};

use nasnet::{config::LoggingSettings, error::AppError};
use tracing_subscriber::{
    EnvFilter,
    filter::{FilterExt, LevelFilter, filter_fn},
    prelude::*,
};

// 此 target 专属于应用边界的致命错误：文件记录事件，终端另行展示一次操作失败摘要。
const ERROR_TARGET: &str = "nasnet::app_error";

/// subscriber 安装成功后的应用句柄；文件由全局 subscriber 持有并同步追加写入。
pub struct Logging {
    file_path: PathBuf,
}

impl Logging {
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    pub fn report_error(&self, error: &AppError) {
        // 只记录安全的顶层展示；底层 TOML source 可能携带原始输入，不递归打印。
        tracing::error!(target: ERROR_TARGET, kind = error.kind(), error = %error, "application failed");
        // 操作失败反馈不受日志过滤控制；终端层排除此 target，避免重复展示。
        eprintln!("{error}");
    }
}

/// 先读取主配置，再调用一次；RUST_LOG 仅覆盖终端层，不影响文件层。
pub fn init(settings: &LoggingSettings, config_directory: &Path) -> Result<Logging, AppError> {
    settings.validate()?;
    let console_filter = match std::env::var("RUST_LOG") {
        Ok(value) => EnvFilter::try_new(value)
            .map_err(|_| AppError::Logging("invalid RUST_LOG directive".to_owned()))?,
        Err(std::env::VarError::NotPresent) => EnvFilter::try_new(&settings.console_level)
            .map_err(|_| AppError::Logging("invalid console log level".to_owned()))?,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(AppError::Logging(
                "RUST_LOG must contain valid Unicode".to_owned(),
            ));
        }
    };
    let file_filter: LevelFilter = settings
        .file_level
        .parse()
        .map_err(|_| AppError::Logging("invalid file log level".to_owned()))?;
    let file_path = if settings.file_path.is_absolute() {
        settings.file_path.clone()
    } else {
        config_directory.join(&settings.file_path)
    };
    if let Some(parent) = file_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::Logging(format!(
                "failed to create log directory at {}: {error}",
                parent.display()
            ))
        })?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .map_err(|error| {
            AppError::Logging(format!(
                "failed to open log file at {}: {error}",
                file_path.display()
            ))
        })?;

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(Mutex::new(file))
        .with_ansi(false)
        .with_target(true)
        .with_filter(file_filter);
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_target(false)
        .with_filter(console_filter.and(filter_fn(|metadata| metadata.target() != ERROR_TARGET)));
    // 每层单独过滤。把 INFO 过滤器放在整个 registry 上会同时丢掉文件所需的 TRACE。
    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .try_init()
        .map_err(|_| AppError::Logging("a global subscriber is already installed".to_owned()))?;
    Ok(Logging { file_path })
}
