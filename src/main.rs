//! NASNet 入口：读取配置、初始化日志、运行应用并统一报告失败。

mod app;

fn main() -> std::process::ExitCode {
    let startup = match app::startup::prepare(std::env::args_os().skip(1).collect()) {
        Ok(startup) => startup,
        Err(error) => {
            eprintln!("{error}");
            return std::process::ExitCode::from(error.exit_code() as u8);
        }
    };
    let logging = match app::logging::init(&startup.config.logging, &startup.config_directory) {
        Ok(logging) => logging,
        Err(error) => {
            // subscriber 尚未建立，配置读取与日志初始化失败都直接显示英文摘要。
            eprintln!("{error}");
            return std::process::ExitCode::from(error.exit_code() as u8);
        }
    };
    tracing::info!(
        config_path = %startup.store.config_path().display(),
        log_path = %logging.file_path().display(),
        "application initialized"
    );
    tracing::debug!(
        references = startup.config.devices.len(),
        console_level = %startup.config.logging.console_level,
        file_level = %startup.config.logging.file_level,
        "logging configured"
    );
    match app::run(startup) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            logging.report_error(&error);
            std::process::ExitCode::from(error.exit_code() as u8)
        }
    }
}
