//! 多个集成测试共用的虚构配置。

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub const APP_CONFIG: &str = include_str!("../../examples/configuration/config.toml");
pub const HOME_DEVICE: &str = include_str!("../../examples/configuration/devices/home.toml");

#[allow(dead_code, reason = "CLI 测试只使用 HOME_DEVICE")]
pub const BACKUP_DEVICE: &str = include_str!("../../examples/configuration/devices/backup.toml");
