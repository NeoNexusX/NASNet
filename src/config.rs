//! 配置快照与 TOML 转换；恢复时重新校验。

use crate::model::{Address, Device, ModelError, ProtocolConnection};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt,
    path::{Component, Path, PathBuf},
};

pub const SCHEMA_VERSION: u32 = 1;

/// 可反序列化的候选数据，不代表已经通过业务校验。
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceData {
    pub id: String,
    pub name: String,
    pub addresses: Vec<Address>,
    pub connections: Vec<ProtocolConnection>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigDocument {
    pub schema_version: u32,
    pub device: DeviceData,
}

impl From<&Device> for DeviceData {
    fn from(device: &Device) -> Self {
        DeviceData {
            id: device.id().to_owned(),
            name: device.name().to_owned(),
            addresses: device.addresses().to_vec(),
            connections: device.connections().to_vec(),
        }
    }
}

impl TryFrom<DeviceData> for Device {
    type Error = ModelError;

    fn try_from(data: DeviceData) -> Result<Self, Self::Error> {
        let mut device = Device::new(data.id, data.name);
        for addr in data.addresses {
            device.try_add_address(addr)?;
        }
        for conn in data.connections {
            device.try_add_connection(conn)?;
        }
        Ok(device)
    }
}

/// 主配置只保存软件设置与设备引用，不包含完整设备数据。
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub schema_version: u32,
    pub software: SoftwareSettings,
    pub logging: LoggingSettings,
    pub devices: Vec<DeviceReference>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareSettings {
    pub language: String,
}

/// 两处输出分别过滤；相对文件路径以主配置所在目录为基准。
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingSettings {
    pub console_level: String,
    pub file_level: String,
    pub file_path: PathBuf,
}

impl LoggingSettings {
    pub fn validate(&self) -> Result<(), ConfigError> {
        let valid_level =
            |level: &str| matches!(level, "trace" | "debug" | "info" | "warn" | "error" | "off");
        // Windows 的盘符相对路径和仅含根的路径不能相对配置目录解释。
        let ambiguous_path = !self.file_path.is_absolute()
            && self
                .file_path
                .components()
                .any(|component| matches!(component, Component::Prefix(_) | Component::RootDir));
        if !valid_level(&self.console_level)
            || !valid_level(&self.file_level)
            || self.file_path.as_os_str().is_empty()
            || self.file_path.file_name().is_none()
            || ambiguous_path
        {
            return Err(ConfigError::InvalidLoggingSettings);
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceReference {
    pub id: String,
    pub path: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            software: SoftwareSettings {
                language: "zh-CN".to_owned(),
            },
            logging: LoggingSettings {
                console_level: "info".to_owned(),
                file_level: "trace".to_owned(),
                file_path: "logs/nasnet.log".into(),
            },
            devices: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), ConfigIoError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(ConfigIoError::UnsupportedVersion(self.schema_version));
        }
        if self.software.language.trim().is_empty() {
            return Err(ConfigError::InvalidSoftwareSettings.into());
        }
        self.logging.validate()?;
        let mut ids = HashSet::new();
        let mut paths = HashSet::new();
        for reference in &self.devices {
            if reference.id.trim().is_empty() || !is_managed_device_path(&reference.path) {
                return Err(ConfigError::InvalidDeviceReference.into());
            }
            if !ids.insert(&reference.id) {
                return Err(ConfigError::DuplicateDeviceId(reference.id.clone()).into());
            }
            if !paths.insert(&reference.path) {
                return Err(ConfigError::DuplicateDevicePath.into());
            }
        }
        Ok(())
    }
}

/// 受管理设备路径的共同规则；只检查形式，不访问文件系统。
pub(crate) fn is_managed_device_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .extension()
            .is_some_and(|extension| extension == "toml")
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

pub fn encode_app_config(config: &AppConfig) -> Result<String, ConfigIoError> {
    // TODO P4.1：先 validate，再 toml::to_string_pretty；只编码主配置，不嵌入设备内容。
    let _ = config;
    todo!("P4.1: encode config.toml")
}

pub fn decode_app_config(text: &str) -> Result<AppConfig, ConfigIoError> {
    let config: AppConfig = toml::from_str(text)?;
    config.validate()?;
    Ok(config)
}

/// 输出包含明文密码，只供配置保存使用，不应写入日志。
pub fn encode_device_toml(device: &Device) -> Result<String, ConfigIoError> {
    // TODO P4.2：用 DeviceData::from 和 SCHEMA_VERSION 构造 ConfigDocument，再编码。
    let _ = device;
    todo!("P4.2: encode one device TOML document")
}

pub fn decode_device_toml(text: &str) -> Result<Device, ConfigIoError> {
    // TODO P4.3：解析 ConfigDocument → 检查版本 → Device::try_from，复用模型校验。
    let _ = text;
    todo!("P4.3: decode and validate one device TOML document")
}

/// 软件/日志设置与设备引用的校验错误。
#[derive(Debug)]
pub enum ConfigError {
    DuplicateDeviceId(String),
    InvalidSoftwareSettings,
    InvalidLoggingSettings,
    InvalidDeviceReference,
    DuplicateDevicePath,
    DeviceIdMismatch,
    DeviceReferenceNotFound(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateDeviceId(id) => write!(formatter, "Duplicate device ID: {id}"),
            Self::InvalidSoftwareSettings => write!(formatter, "Invalid software settings"),
            Self::InvalidLoggingSettings => write!(formatter, "Invalid logging settings"),
            Self::InvalidDeviceReference => write!(formatter, "Invalid device reference"),
            Self::DuplicateDevicePath => write!(formatter, "Duplicate device file path"),
            Self::DeviceIdMismatch => write!(formatter, "Device ID does not match its reference"),
            Self::DeviceReferenceNotFound(id) => {
                write!(formatter, "Device reference not found: {id}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// TOML 编解码与配置校验错误；Display 不包含原始配置正文。
#[derive(Debug)]
pub enum ConfigIoError {
    TomlDecode(toml::de::Error),
    TomlEncode(toml::ser::Error),
    Validation(ConfigError),
    ModelValidation(ModelError),
    UnsupportedVersion(u32),
}

impl fmt::Display for ConfigIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TomlDecode(_) => write!(formatter, "Invalid TOML configuration"),
            Self::TomlEncode(_) => write!(formatter, "Failed to encode TOML configuration"),
            Self::ModelValidation(error) => {
                write!(formatter, "Configuration validation failed: {error}")
            }
            Self::Validation(error) => {
                write!(formatter, "Configuration validation failed: {error}")
            }
            Self::UnsupportedVersion(version) => {
                write!(formatter, "Unsupported configuration version: {version}")
            }
        }
    }
}

impl std::error::Error for ConfigIoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TomlDecode(error) => Some(error),
            Self::TomlEncode(error) => Some(error),
            Self::Validation(error) => Some(error),
            Self::ModelValidation(error) => Some(error),
            Self::UnsupportedVersion(_) => None,
        }
    }
}

impl From<ConfigError> for ConfigIoError {
    fn from(error: ConfigError) -> Self {
        Self::Validation(error)
    }
}

impl From<ModelError> for ConfigIoError {
    fn from(error: ModelError) -> Self {
        Self::ModelValidation(error)
    }
}

impl From<toml::de::Error> for ConfigIoError {
    fn from(error: toml::de::Error) -> Self {
        Self::TomlDecode(error)
    }
}
impl From<toml::ser::Error> for ConfigIoError {
    fn from(error: toml::ser::Error) -> Self {
        Self::TomlEncode(error)
    }
}
