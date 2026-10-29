//! 全项目的错误类型与错误转换；模型和存储返回错误，应用边界负责记录与展示。

use std::{fmt, io, path::PathBuf};

/// 配置校验或模型操作失败的具体原因。
#[derive(Debug)]
pub enum ConfigError {
    DuplicateDeviceId(String),
    InvalidSoftwareSettings,
    InvalidDeviceReference,
    DuplicateDevicePath,
    DeviceIdMismatch,
    DeviceReferenceNotFound(String),
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

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSoftwareSettings => write!(formatter, "Invalid software settings"),
            Self::InvalidDeviceReference => write!(formatter, "Invalid device reference"),
            Self::DuplicateDevicePath => write!(formatter, "Duplicate device file path"),
            Self::DeviceIdMismatch => write!(formatter, "Device ID does not match its reference"),
            Self::DeviceReferenceNotFound(id) => {
                write!(formatter, "Device reference not found: {id}")
            }
            Self::DuplicateDeviceId(id) => write!(formatter, "Duplicate device ID: {id}"),
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

impl std::error::Error for ConfigError {}

/// JSON 复习与 TOML 配置分开表达；Display 不包含原始配置正文。
#[derive(Debug)]
pub enum ConfigIoError {
    Json(serde_json::Error),
    TomlDecode(toml::de::Error),
    TomlEncode(toml::ser::Error),
    Validation(ConfigError),
    UnsupportedVersion(u32),
}

impl fmt::Display for ConfigIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TomlDecode(_) => write!(formatter, "Invalid TOML configuration"),
            Self::TomlEncode(_) => write!(formatter, "Failed to encode TOML configuration"),
            Self::Json(error) => write!(
                formatter,
                "Invalid JSON configuration at line {}, column {}",
                error.line(),
                error.column()
            ),
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
            Self::Json(error) => Some(error),
            Self::TomlDecode(error) => Some(error),
            Self::TomlEncode(error) => Some(error),
            Self::Validation(error) => Some(error),
            Self::UnsupportedVersion(_) => None,
        }
    }
}

impl From<serde_json::Error> for ConfigIoError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<ConfigError> for ConfigIoError {
    fn from(error: ConfigError) -> Self {
        Self::Validation(error)
    }
}

/// 文件存储错误包含操作与路径，便于应用边界定位失败阶段。
#[derive(Debug)]
pub enum StoreError {
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Config(ConfigIoError),
    ConfigFile {
        path: PathBuf,
        source: ConfigIoError,
    },
    InvalidConfigPath,
}

impl StoreError {
    pub fn config(path: impl Into<PathBuf>, source: ConfigIoError) -> Self {
        Self::ConfigFile {
            path: path.into(),
            source,
        }
    }
    pub fn io(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            operation,
            path: path.into(),
            source,
        }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                source,
            } => write!(
                formatter,
                "Failed to {operation} at {}: {source}",
                path.display()
            ),
            Self::Config(error) => write!(formatter, "Stored configuration is invalid: {error}"),
            Self::ConfigFile { path, source } => write!(
                formatter,
                "Invalid configuration at {}: {source}",
                path.display()
            ),
            Self::InvalidConfigPath => write!(formatter, "Configuration path must identify a file"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Config(error) => Some(error),
            Self::ConfigFile { source, .. } => Some(source),
            Self::InvalidConfigPath => None,
        }
    }
}

impl From<ConfigIoError> for StoreError {
    fn from(error: ConfigIoError) -> Self {
        Self::Config(error)
    }
}

/// 应用入口统一接收的错误；只有应用边界决定日志字段、展示和退出码。
#[derive(Debug)]
pub enum AppError {
    Config(ConfigIoError),
    Store(StoreError),
    Io(io::Error),
    UnknownCourse(String),
    InvalidArguments(&'static str),
    Logging(String),
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Config(_) => "config",
            Self::Store(_) => "store",
            Self::Io(_) => "io",
            Self::UnknownCourse(_) => "unknown_course",
            Self::InvalidArguments(_) => "invalid_arguments",
            Self::Logging(_) => "logging",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::UnknownCourse(_) | Self::InvalidArguments(_) => 2,
            _ => 1,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "Configuration operation failed: {error}"),
            Self::Store(error) => write!(formatter, "Configuration storage failed: {error}"),
            Self::Io(error) => write!(formatter, "I/O operation failed: {error}"),
            Self::UnknownCourse(course) => write!(formatter, "Unknown course: {course}"),
            Self::InvalidArguments(message) => write!(formatter, "Invalid arguments: {message}"),
            Self::Logging(message) => write!(formatter, "Logging initialization failed: {message}"),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::Store(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::UnknownCourse(_) | Self::InvalidArguments(_) | Self::Logging(_) => None,
        }
    }
}

impl From<ConfigIoError> for AppError {
    fn from(error: ConfigIoError) -> Self {
        Self::Config(error)
    }
}

impl From<StoreError> for AppError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ConfigError> for AppError {
    fn from(error: ConfigError) -> Self {
        Self::Config(ConfigIoError::Validation(error))
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
