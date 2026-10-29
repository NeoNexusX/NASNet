//! 应用边界的日志订阅与错误报告；模型和存储只返回 Result。

use nasnet::error::AppError;
use tracing_subscriber::EnvFilter;

pub fn init() -> Result<(), AppError> {
    let filter = match std::env::var("RUST_LOG") {
        Ok(value) => EnvFilter::try_new(value)
            .map_err(|_| AppError::Logging("invalid RUST_LOG directive".to_owned()))?,
        Err(std::env::VarError::NotPresent) => EnvFilter::new("info"),
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(AppError::Logging(
                "RUST_LOG must contain valid Unicode".to_owned(),
            ));
        }
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_target(false)
        .try_init()
        .map_err(|_| AppError::Logging("a global subscriber is already installed".to_owned()))
}

pub fn report_error(error: &AppError) {
    // 只记录安全的顶层展示；底层 serde source 可能携带原始输入，不递归打印。
    if tracing::enabled!(tracing::Level::ERROR) {
        tracing::error!(kind = error.kind(), error = %error, "application failed");
    } else {
        // 日志过滤不能让用户看不到操作失败；仍只在应用边界展示一次。
        eprintln!("{error}");
    }
}
