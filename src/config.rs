//! 配置快照、已完成的 JSON 复习与待实现的 TOML 转换；恢复时重新校验。

pub use crate::error::ConfigIoError;
use crate::model::{Address, ConfigError, Device, ProtocolConnection};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

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
    type Error = ConfigError;

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

/// 输出包含明文密码，只供配置保存使用，不应写入日志。
pub fn encode_device(device: &Device) -> Result<String, ConfigIoError> {
    Ok(serde_json::to_string_pretty(&ConfigDocument {
        schema_version: SCHEMA_VERSION,
        device: DeviceData::from(device),
    })?)
}

pub fn decode_device(json: &str) -> Result<Device, ConfigIoError> {
    let configdocument: ConfigDocument = serde_json::from_str::<ConfigDocument>(json)?;
    if configdocument.schema_version != SCHEMA_VERSION {
        return Err(ConfigIoError::UnsupportedVersion(
            configdocument.schema_version,
        ));
    }
    let device = Device::try_from(configdocument.device)?;
    Ok(device)
}

/// 主配置只保存软件设置与设备引用，不包含完整设备数据。
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    pub schema_version: u32,
    pub software: SoftwareSettings,
    pub devices: Vec<DeviceReference>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareSettings {
    pub language: String,
    pub log_level: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceReference {
    pub id: String,
    pub path: std::path::PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            software: SoftwareSettings {
                language: "zh-CN".to_owned(),
                log_level: "info".to_owned(),
            },
            devices: Vec::new(),
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), ConfigIoError> {
        // TODO 4.2：检查主版本、软件设置、引用 ID/路径非空且不重复。
        // log_level 只接受 trace/debug/info/warn/error/off；不读取设备文件。
        let _ = self;
        todo!("TODO 4.2: validate software settings and device references")
    }
}

pub fn encode_app_config(config: &AppConfig) -> Result<String, ConfigIoError> {
    // TODO 4.3：先 validate，再 toml::to_string_pretty；不把设备内容塞回主文件。
    let _ = config;
    todo!("TODO 4.3: encode config.toml")
}

pub fn decode_app_config(text: &str) -> Result<AppConfig, ConfigIoError> {
    // TODO 4.4：toml::from_str 解析 AppConfig，再 validate。
    let _ = text;
    todo!("TODO 4.4: decode and validate config.toml")
}

pub fn encode_device_toml(device: &Device) -> Result<String, ConfigIoError> {
    // TODO 4.5：复用已验收 DeviceData::from 和 ConfigDocument，编码单设备 TOML。
    let _ = device;
    todo!("TODO 4.5: encode one device TOML document")
}

pub fn decode_device_toml(text: &str) -> Result<Device, ConfigIoError> {
    // TODO 4.6：解析 ConfigDocument，检查版本，再 Device::try_from；保留模型校验。
    let _ = text;
    todo!("TODO 4.6: decode and validate one device TOML document")
}
