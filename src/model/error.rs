//! 模型自己的错误；不依赖配置、文件或应用模块。

use std::fmt;

/// 设备、地址与协议配置的校验或操作错误。
#[derive(Debug)]
pub enum ModelError {
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
    EmptySmbPassword,
    InvalidPort,
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAddressId => write!(formatter, "Address ID must not be empty"),
            Self::EmptyHost => write!(formatter, "Host must not be empty or whitespace only"),
            Self::HostContainsWhitespace => write!(formatter, "Host must not contain whitespace"),
            Self::HostContainsSchemeOrPath => {
                write!(
                    formatter,
                    "Host must be an IP address or domain without a scheme or path"
                )
            }
            Self::DuplicateAddressId(id) => write!(formatter, "Duplicate address ID: {id}"),
            Self::AddressNotFound(id) => write!(formatter, "Address ID not found: {id}"),
            Self::EmptyConnectionId => write!(formatter, "Connection ID must not be empty"),
            Self::DuplicateConnectionId(id) => write!(formatter, "Duplicate connection ID: {id}"),
            Self::ConnectionNotFound(id) => write!(formatter, "Connection ID not found: {id}"),
            Self::InvalidNfsExportPath => {
                write!(
                    formatter,
                    "NFS export path must start with / and must not contain null characters"
                )
            }
            Self::InvalidSmbShareName => {
                write!(
                    formatter,
                    "SMB share name must not be empty or contain path separators or null characters"
                )
            }
            Self::InvalidSmbBasePath => {
                write!(
                    formatter,
                    "SMB base path must be relative to the share and must not contain null characters"
                )
            }
            Self::EmptySmbUsername => write!(formatter, "SMB username must not be empty"),
            Self::EmptySmbPassword => write!(formatter, "SMB password must not be empty"),
            Self::InvalidPort => write!(formatter, "An explicit service port must not be zero"),
        }
    }
}

impl std::error::Error for ModelError {}
