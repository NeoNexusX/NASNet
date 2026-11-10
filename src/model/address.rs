//! 地址模型及本地格式校验，不查询 DNS 或连接网络。

use serde::{Deserialize, Serialize};

use crate::model::ModelError;

/// IP/域名与优先级记录；可公开构造候选值，保存时由设备校验。
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Address {
    pub id: String,
    pub host: String,
    /// 数值越小，优先级越高；同优先级按保存顺序排列。
    pub priority: u32,
}

impl Address {
    /// 只检查本地格式，保留输入原值。
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.id.trim().is_empty() {
            return Err(ModelError::EmptyAddressId);
        }
        self.validate_host()?;
        Ok(())
    }

    fn validate_host(&self) -> Result<(), ModelError> {
        let host = &self.host;
        if host.trim().is_empty() {
            return Err(ModelError::EmptyHost);
        }
        if host.chars().any(|c| c.is_whitespace()) {
            return Err(ModelError::HostContainsWhitespace);
        }
        if host.contains("://") || host.contains('/') || host.contains('\\') {
            return Err(ModelError::HostContainsSchemeOrPath);
        }
        Ok(())
    }
}
