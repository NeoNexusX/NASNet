//! 二进制应用：准备启动 → 安装日志 → 打印配置与选中的设备。所有失败都在 run 中向终端报告一次。

mod logging;
mod startup;

use std::{ffi::OsString, fmt, io, process::ExitCode};

use nasnet::{config::ConfigError, store::StoreError};

use self::startup::Startup;

/// 唯一打印失败摘要、决定退出码的地方。
pub fn run(arguments: impl IntoIterator<Item = OsString>) -> ExitCode {
    match execute(arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(error.exit_code())
        }
    }
}

/// 每一步失败都立即返回，后面的步骤不会执行。
fn execute(arguments: impl IntoIterator<Item = OsString>) -> Result<(), AppError> {
    let startup = startup::prepare(arguments)?;
    logging::init(&startup.config.logging, &startup.log_path)?;
    tracing::info!(
        config_path = %startup.store.config_path().display(),
        log_path = %startup.log_path.display(),
        references = startup.config.devices.len(),
        "application initialized"
    );
    tracing::debug!(
        console_level = %startup.config.logging.console_level,
        file_level = %startup.config.logging.file_level,
        "logging configured"
    );
    let result = show(&startup);
    if let Err(error) = &result {
        logging::record_error(error);
    }
    result
}

/// 打印主配置；指定了 --device 时只读取那一个设备文件。
fn show(startup: &Startup) -> Result<(), AppError> {
    let span = tracing::info_span!("application");
    let _entered = span.enter();
    let config = &startup.config;
    println!("Language: {}", config.software.language);
    for reference in &config.devices {
        tracing::trace!(device_id = %reference.id, path = %reference.path.display(), "device reference listed");
        println!("{} -> {}", reference.id, reference.path.display());
    }
    if let Some(id) = startup.selected_id.as_deref() {
        let reference = config
            .devices
            .iter()
            .find(|reference| reference.id == id)
            .ok_or_else(|| ConfigError::DeviceReferenceNotFound(id.to_owned()))?;
        let device = startup.store.load_device(reference)?;
        println!("Selected device: {} ({})", device.name(), device.id());
        tracing::info!(device_id = id, "selected device loaded");
    }
    tracing::info!("application completed");
    Ok(())
}

/// 应用边界汇总的错误；核心库不依赖它。
#[derive(Debug)]
enum AppError {
    InvalidArguments(&'static str),
    Io(io::Error),
    Config(ConfigError),
    Store(StoreError),
    Logging(String),
}

impl AppError {
    /// 参数错误退出码为 2，其他失败为 1。
    fn exit_code(&self) -> u8 {
        match self {
            Self::InvalidArguments(_) => 2,
            _ => 1,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArguments(message) => write!(formatter, "Invalid arguments: {message}"),
            Self::Io(error) => write!(formatter, "I/O operation failed: {error}"),
            Self::Config(error) => write!(formatter, "Configuration error: {error}"),
            Self::Store(error) => write!(formatter, "Configuration storage failed: {error}"),
            Self::Logging(message) => write!(formatter, "Logging initialization failed: {message}"),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Config(error) => Some(error),
            Self::Store(error) => Some(error),
            Self::InvalidArguments(_) | Self::Logging(_) => None,
        }
    }
}

impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ConfigError> for AppError {
    fn from(error: ConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<StoreError> for AppError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}
