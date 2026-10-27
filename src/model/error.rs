//! 设备、地址和协议配置使用的统一错误类型及其展示。

use std::fmt;

/// 配置校验或模型操作失败的具体原因。
#[derive(Debug)]
pub enum ConfigError {
    EmptyAddressId,
    EmptyHost,
    HostContainsWhitespace,
    HostContainsSchemeOrPath,
    DuplicateAddressId(String),
    AddressNotFound(String),
    EmptyConnectionId,
    DuplicateConnectionId(String),
    ConnectionNotFound(String),
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
            Self::AddressNotFound(id) => write!(formatter, "地址 ID 不存在：{id}"),
            Self::EmptyConnectionId => write!(formatter, "连接 ID 不能为空"),
            Self::DuplicateConnectionId(id) => write!(formatter, "连接 ID 已存在：{id}"),
            Self::ConnectionNotFound(id) => write!(formatter, "连接 ID 不存在：{id}"),
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
