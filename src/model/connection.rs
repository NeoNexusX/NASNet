//! 协议连接记录与配置校验分派；地址候选列表由所属设备提供。

mod nfs;
mod smb;

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

pub use self::nfs::{NfsAuthSys, NfsConfig, NfsVersion};
pub use self::smb::{SmbAuth, SmbConfig, SmbDialect};

/// 每条连接恰好拥有一种协议的配置。
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "protocol", content = "settings", rename_all = "snake_case")]
pub enum ConnectionConfig {
    Nfs(NfsConfig),
    Smb(SmbConfig),
}

/// 连接归所属设备管理，不重复保存设备的 Host 或地址优先级。
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolConnection {
    pub id: String,
    pub config: ConnectionConfig,
}

impl ProtocolConnection {
    /// 检查连接 ID，并按实际协议分派配置校验。
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.id.trim().is_empty() {
            return Err(ConfigError::EmptyConnectionId);
        }
        match &self.config {
            ConnectionConfig::Nfs(nfs_config) => nfs_config.validate(),
            ConnectionConfig::Smb(smb_config) => smb_config.validate(),
        }
    }
}
