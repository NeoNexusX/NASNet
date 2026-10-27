//! 地址模型及本地格式校验，不查询 DNS 或连接网络。

use super::error::ConfigError;

/// IP/域名与优先级记录；可公开构造候选值，保存时由设备校验。
pub struct Address {
    pub id: String,
    pub host: String,
    /// 数值越小，优先级越高；同优先级按保存顺序排列。
    pub priority: u32,
}

impl Address {
    /// 只读检查地址，不自动修改 ID 或 Host，也不解析域名或连接网络。
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.id.trim().is_empty() {
            return Err(ConfigError::EmptyAddressId);
        }
        validate_host(&self.host)?;
        Ok(())
    }
}

/// 基础 Host 校验：不自动 trim 输入，不查询 DNS，不判断服务器能否访问。
pub fn validate_host(host: &str) -> Result<(), ConfigError> {
    let trim_result = host.trim();
    if trim_result.is_empty() {
        return Err(ConfigError::EmptyHost);
    }
    if host.chars().any(|c| c.is_whitespace()) {
        return Err(ConfigError::HostContainsWhitespace);
    }
    if host.contains("://") || host.contains('/') || host.contains('\\') {
        return Err(ConfigError::HostContainsSchemeOrPath);
    }
    Ok(())
}
