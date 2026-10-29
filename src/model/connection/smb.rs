//! SMB 配置、账号身份和本地格式校验。

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

/// 自动协商，或限制在对应的 SMB 版本系列；具体方言由后端协商。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SmbDialect {
    Auto,
    Smb2,
    Smb3,
}

/// 当前提供账号认证，密码按原值保存在配置中；展示和日志不输出密码。
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmbAuth {
    pub username: String,
    pub domain: Option<String>,
    pub password: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmbConfig {
    pub share_name: String,
    /// None 或空字符串表示共享根目录；其他值按共享内相对路径解释。
    pub base_path: Option<String>,
    pub port: Option<u16>,
    pub authentication: SmbAuth,
    pub dialect: SmbDialect,
}

impl SmbConfig {
    /// 检查共享、账号和端口格式，不读取密码或检查远程权限。
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.share_name.trim().is_empty()
            || self.share_name.contains('/')
            || self.share_name.contains('\\')
            || self.share_name.contains('\0')
        {
            return Err(ConfigError::InvalidSmbShareName);
        }
        if let Some(path) = &self.base_path {
            if path.starts_with('/') || path.starts_with('\\') || path.contains('\0') {
                return Err(ConfigError::InvalidSmbBasePath);
            }
        }
        if self.authentication.username.trim().is_empty() {
            return Err(ConfigError::EmptySmbUsername);
        }
        if self.authentication.password.is_empty() {
            return Err(ConfigError::EmptySmbPassword);
        }
        if self.port == Some(0) {
            return Err(ConfigError::InvalidPort);
        }
        Ok(())
    }
}
