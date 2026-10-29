//! 多个集成测试共用的虚构主配置；不在测试中改写 examples 下的文件。

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub const APP_CONFIG: &str = include_str!("../../examples/configuration/config.toml");
