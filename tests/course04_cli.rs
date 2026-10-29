mod common;

use common::{APP_CONFIG, TestResult};
use nasnet::config::{AppConfig, decode_app_config};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

/// 每例一个目录，每次运行一个子进程；不修改测试进程的环境、cwd 或 subscriber。
struct Cli {
    directory: tempfile::TempDir,
    config_path: PathBuf,
    working_directory: PathBuf,
}

impl Cli {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let config_directory = directory.path().join("配置 with spaces");
        fs::create_dir(&config_directory)?;
        let config_path = config_directory.join("settings.toml");
        fs::write(&config_path, APP_CONFIG)?;
        let working_directory = directory.path().join("working-directory");
        fs::create_dir(&working_directory)?;
        Ok(Self {
            directory,
            config_path,
            working_directory,
        })
    }

    fn raw_command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_nasnet"));
        command
            .current_dir(&self.working_directory)
            .env_remove("RUST_LOG");
        command
    }

    fn command(&self) -> Command {
        let mut command = self.raw_command();
        command.arg("--config").arg(&self.config_path);
        command
    }

    fn write_config(&self, config: &AppConfig) -> TestResult {
        // 仅生成测试输入；不调用尚未完成的项目 encode_app_config。
        fs::write(&self.config_path, toml::to_string_pretty(config)?)?;
        Ok(())
    }

    fn log_path(&self) -> PathBuf {
        self.config_path.parent().unwrap().join("logs/nasnet.log")
    }

    fn log(&self) -> Result<String, std::io::Error> {
        fs::read_to_string(self.log_path())
    }

    fn portable_binary(&self) -> Result<PathBuf, std::io::Error> {
        let directory = self.directory.path().join("portable");
        fs::create_dir(&directory)?;
        let source = std::path::Path::new(env!("CARGO_BIN_EXE_nasnet"));
        let executable = directory.join(source.file_name().unwrap());
        fs::copy(source, &executable)?;
        Ok(executable)
    }
}

fn succeeds(output: &Output) {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn file_trace_and_console_info_are_filtered_independently() -> TestResult {
    let fixture = Cli::new()?;
    let output = fixture.command().output()?;
    succeeds(&output);
    let stderr = String::from_utf8(output.stderr)?;
    let stdout = String::from_utf8(output.stdout)?;
    let log = fixture.log()?;
    assert!(stderr.contains("application initialized"));
    assert!(!stderr.contains("logging configured"));
    assert!(!stderr.contains("device reference listed"));
    assert!(log.contains("logging configured") && log.contains("DEBUG"));
    assert!(log.contains("device reference listed") && log.contains("TRACE"));
    assert!(!log.contains("\u{1b}["));
    assert!(stdout.contains("nas-home ->"));
    assert!(!stdout.contains("application initialized"));
    assert!(!log.contains("Language: zh-CN"));
    Ok(())
}

#[test]
fn rust_log_overrides_console_settings_without_changing_file_filter() -> TestResult {
    let fixture = Cli::new()?;
    let mut config = decode_app_config(APP_CONFIG)?;
    config.logging.console_level = "warn".into();
    fixture.write_config(&config)?;
    let output = fixture.command().env("RUST_LOG", "debug").output()?;
    succeeds(&output);
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("logging configured"));
    assert!(!stderr.contains("device reference listed"));
    assert!(fixture.log()?.contains("device reference listed"));
    Ok(())
}

#[test]
fn disabling_console_logging_keeps_file_events_and_command_results() -> TestResult {
    let fixture = Cli::new()?;
    let output = fixture.command().env("RUST_LOG", "off").output()?;
    succeeds(&output);
    assert!(output.stderr.is_empty());
    assert!(String::from_utf8(output.stdout)?.contains("nas-home"));
    assert!(fixture.log()?.contains("device reference listed"));
    Ok(())
}

#[test]
fn file_warn_does_not_suppress_console_debug() -> TestResult {
    let fixture = Cli::new()?;
    let mut config = decode_app_config(APP_CONFIG)?;
    config.logging.console_level = "debug".into();
    config.logging.file_level = "warn".into();
    fixture.write_config(&config)?;
    let output = fixture.command().output()?;
    succeeds(&output);
    assert!(String::from_utf8(output.stderr)?.contains("logging configured"));
    assert!(fixture.log()?.is_empty());
    let output = fixture.command().args(["--device", "missing"]).output()?;
    assert_eq!(output.status.code(), Some(1));
    let log = fixture.log()?;
    assert!(log.contains("ERROR") && log.contains("application failed"));
    assert!(!log.contains("logging configured"));
    Ok(())
}

#[test]
fn file_off_does_not_suppress_console_info() -> TestResult {
    let fixture = Cli::new()?;
    let mut config = decode_app_config(APP_CONFIG)?;
    config.logging.file_level = "off".into();
    fixture.write_config(&config)?;
    let output = fixture.command().output()?;
    succeeds(&output);
    assert!(String::from_utf8(output.stderr)?.contains("application initialized"));
    assert!(fixture.log()?.is_empty());
    Ok(())
}

#[test]
fn log_files_are_appended_across_process_runs() -> TestResult {
    let fixture = Cli::new()?;
    succeeds(&fixture.command().output()?);
    let before = fs::read(fixture.log_path())?;
    succeeds(&fixture.command().output()?);
    let after = fs::read(fixture.log_path())?;
    assert!(after.starts_with(&before) && after.len() > before.len());
    assert_eq!(
        String::from_utf8(after)?
            .matches("application initialized")
            .count(),
        2
    );
    Ok(())
}

#[test]
fn fatal_errors_appear_once_on_console_and_once_in_the_file() -> TestResult {
    let fixture = Cli::new()?;
    for directive in ["off", "trace", "nasnet=off", "off,nasnet::app_error=trace"] {
        let output = fixture
            .command()
            .env("RUST_LOG", directive)
            .args(["--device", "missing"])
            .output()?;
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            String::from_utf8(output.stderr)?
                .matches("Device reference not found: missing")
                .count(),
            1
        );
    }
    assert_eq!(fixture.log()?.matches("application failed").count(), 4);
    Ok(())
}

#[test]
fn invalid_environment_filter_fails_before_opening_a_log_file() -> TestResult {
    let fixture = Cli::new()?;
    let output = fixture
        .command()
        .env("RUST_LOG", "nasnet=not_a_level")
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr)?.contains("invalid RUST_LOG directive"));
    assert!(!fixture.log_path().exists());
    Ok(())
}

#[test]
fn explicit_configuration_accepts_spaces_and_option_order() -> TestResult {
    let fixture = Cli::new()?;
    let original = fs::read(&fixture.config_path)?;
    let output = fixture
        .raw_command()
        .args(["--device", "missing", "--config"])
        .arg(&fixture.config_path)
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr)?.contains("Device reference not found: missing"));
    assert_eq!(fs::read(&fixture.config_path)?, original);
    assert!(fixture.log_path().exists());
    assert!(!fixture.working_directory.join("logs").exists());
    assert!(
        !fixture
            .config_path
            .parent()
            .unwrap()
            .join("devices")
            .exists()
    );
    Ok(())
}

#[test]
fn relative_cli_configuration_path_is_resolved_from_launch_directory() -> TestResult {
    let fixture = Cli::new()?;
    let relative = fixture.config_path.strip_prefix(fixture.directory.path())?;
    let output = fixture
        .raw_command()
        .current_dir(fixture.directory.path())
        .arg("--config")
        .arg(relative)
        .output()?;
    succeeds(&output);
    assert!(fixture.log_path().exists());
    Ok(())
}

#[test]
fn absolute_log_paths_are_supported() -> TestResult {
    let fixture = Cli::new()?;
    let mut config = decode_app_config(APP_CONFIG)?;
    config.logging.file_path = fixture.directory.path().join("absolute/events.log");
    fixture.write_config(&config)?;
    succeeds(&fixture.command().output()?);
    assert!(config.logging.file_path.is_file());
    assert!(!fixture.log_path().exists());
    Ok(())
}

#[test]
fn default_missing_configuration_uses_the_executable_directory() -> TestResult {
    let fixture = Cli::new()?;
    let executable = fixture.portable_binary()?;
    let portable = executable.parent().unwrap();
    fs::write(fixture.working_directory.join("config.toml"), "invalid = [")?;
    let output = Command::new(&executable)
        .current_dir(&fixture.working_directory)
        .env_remove("RUST_LOG")
        .output()?;
    succeeds(&output);
    assert!(portable.join("logs/nasnet.log").exists());
    assert!(!portable.join("config.toml").exists());
    assert!(!fixture.working_directory.join("logs").exists());
    Ok(())
}

#[test]
fn default_existing_configuration_is_loaded_beside_the_executable() -> TestResult {
    let fixture = Cli::new()?;
    let executable = fixture.portable_binary()?;
    let portable = executable.parent().unwrap();
    fs::copy(&fixture.config_path, portable.join("config.toml"))?;
    let output = Command::new(&executable)
        .current_dir(&fixture.working_directory)
        .env_remove("RUST_LOG")
        .output()?;
    succeeds(&output);
    assert!(String::from_utf8(output.stdout)?.contains("nas-backup"));
    assert!(portable.join("logs/nasnet.log").exists());
    Ok(())
}

#[test]
fn explicit_missing_configuration_is_not_replaced_with_defaults() -> TestResult {
    let fixture = Cli::new()?;
    let output = fixture
        .raw_command()
        .arg("--config")
        .arg(fixture.directory.path().join("missing.toml"))
        .output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr)?.contains("read configuration"));
    assert!(!fixture.log_path().exists());
    Ok(())
}

#[test]
fn malformed_configuration_is_reported_without_input_or_file_changes() -> TestResult {
    let fixture = Cli::new()?;
    let invalid = "TEST_SECRET_SENTINEL = [";
    fs::write(&fixture.config_path, invalid)?;
    let output = fixture.command().output()?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("Invalid TOML configuration"));
    assert!(!stderr.contains("TEST_SECRET_SENTINEL"));
    assert!(output.stdout.is_empty());
    assert!(!fixture.log_path().exists());
    assert_eq!(fs::read_to_string(&fixture.config_path)?, invalid);
    Ok(())
}

#[test]
fn log_paths_cannot_overwrite_the_main_configuration() -> TestResult {
    let fixture = Cli::new()?;
    for path in [
        PathBuf::from("settings.toml"),
        PathBuf::from("logs/../settings.toml"),
        fixture.config_path.clone(),
    ] {
        let mut config = decode_app_config(APP_CONFIG)?;
        config.logging.file_path = path;
        fixture.write_config(&config)?;
        let original = fs::read(&fixture.config_path)?;
        let output = fixture.command().output()?;
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8(output.stderr)?.contains("log file path conflicts"));
        assert_eq!(fs::read(&fixture.config_path)?, original);
    }
    Ok(())
}

#[test]
fn log_paths_cannot_overwrite_registered_device_files() -> TestResult {
    let fixture = Cli::new()?;
    let mut config = decode_app_config(APP_CONFIG)?;
    config.logging.file_path = "devices/home.toml".into();
    fixture.write_config(&config)?;
    let device_path = fixture
        .config_path
        .parent()
        .unwrap()
        .join("devices/home.toml");
    fs::create_dir(device_path.parent().unwrap())?;
    fs::write(&device_path, "DEVICE_SENTINEL")?;
    let output = fixture.command().output()?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read_to_string(device_path)?, "DEVICE_SENTINEL");
    Ok(())
}

#[test]
fn a_directory_at_the_log_file_path_is_an_initialization_error() -> TestResult {
    let fixture = Cli::new()?;
    fs::create_dir_all(fixture.log_path())?;
    let output = fixture.command().output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr)?.contains("failed to open log file"));
    Ok(())
}

#[test]
fn invalid_and_removed_demo_arguments_are_rejected_before_logging() -> TestResult {
    let fixture = Cli::new()?;
    let invalid: &[&[&str]] = &[
        &["--config"],
        &["--config", "--device"],
        &["--device"],
        &["--device", ""],
        &["--device", "a", "--device", "b"],
        &["--config", "a", "--config", "b"],
        &["--unknown"],
        &["course01"],
        &["lesson06"],
        &["--intro"],
        &["--demo"],
    ];
    for arguments in invalid {
        let output = fixture.raw_command().args(*arguments).output()?;
        assert_eq!(output.status.code(), Some(2), "arguments: {arguments:?}");
        assert_eq!(
            String::from_utf8(output.stderr)?
                .matches("Invalid arguments:")
                .count(),
            1
        );
    }
    assert!(!fixture.log_path().exists());
    Ok(())
}
