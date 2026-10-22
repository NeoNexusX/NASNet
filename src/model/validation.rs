//! 配置校验的错误类型和 Host 检查入口。
//!
//! 基础格式校验不包含 DNS 解析或网络探测。

use std::fmt;

/// 配置校验失败的具体原因。
#[derive(Debug)]
#[allow(dead_code)] // TODO 10～12 完成后，移除未构造错误变体的临时豁免。
pub enum ConfigError {
    EmptyAddressId,
    EmptyHost,
    HostContainsWhitespace,
    HostContainsSchemeOrPath,
    DuplicateAddressId(String),
    EmptyConnectionId,
    DuplicateConnectionId(String),
    InvalidNfsExportPath,
    InvalidSmbShareName,
    InvalidSmbBasePath,
    EmptySmbUsername,
    EmptyPasswordRef,
    InvalidPort,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAddressId => write!(formatter, "地址 ID 不能为空"),
            Self::EmptyHost => write!(formatter, "Host 不能为空或只有空白字符"),
            Self::HostContainsWhitespace => write!(formatter, "Host 不能包含空白字符"),
            Self::HostContainsSchemeOrPath => {
                write!(
                    formatter,
                    "Host 不应包含协议前缀或路径，请单独填写 IP 或域名"
                )
            }
            Self::DuplicateAddressId(id) => write!(formatter, "地址 ID 已存在：{id}"),
            Self::EmptyConnectionId => write!(formatter, "连接 ID 不能为空"),
            Self::DuplicateConnectionId(id) => write!(formatter, "连接 ID 已存在：{id}"),
            Self::InvalidNfsExportPath => {
                write!(formatter, "NFS 导出路径必须以 / 开头且不含空字符")
            }
            Self::InvalidSmbShareName => {
                write!(
                    formatter,
                    "SMB 共享名不能为空，且不能包含路径分隔符或空字符"
                )
            }
            Self::InvalidSmbBasePath => {
                write!(formatter, "SMB 起始目录必须是共享内的相对路径且不含空字符")
            }
            Self::EmptySmbUsername => write!(formatter, "SMB 用户名不能为空"),
            Self::EmptyPasswordRef => write!(formatter, "SMB 密码凭据引用不能为空"),
            Self::InvalidPort => write!(formatter, "显式指定的服务端口不能为 0"),
        }
    }
}

impl std::error::Error for ConfigError {}

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
