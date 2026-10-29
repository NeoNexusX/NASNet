//! NFS 配置、AUTH_SYS 身份和本地格式校验。

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

/// 请求使用的 NFS 版本，实际支持情况由协议后端检查。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NfsVersion {
    V3,
    V4,
}

/// NFS AUTH_SYS 请求身份，与远程文件属主及服务器授权分开管理。
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NfsAuthSys {
    pub uid: u32,
    pub gid: u32,
    pub supplementary_groups: Vec<u32>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NfsConfig {
    pub export_path: String,
    pub version: NfsVersion,
    pub port: Option<u16>,
    pub authentication: NfsAuthSys,
}

impl NfsConfig {
    /// 检查本地配置格式，不探测服务器导出或后端能力。
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.export_path.contains('\0') || !self.export_path.starts_with('/') {
            return Err(ConfigError::InvalidNfsExportPath);
        }
        if self.port == Some(0) {
            return Err(ConfigError::InvalidPort);
        }
        Ok(())
    }
}
