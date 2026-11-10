//! 远程连接和目录读取契约。

use std::fmt;

use crate::model::{Device, ProtocolConnection};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    /// 条目名，不含父目录。
    pub name: String,
    pub kind: EntryKind,
}

/// 符号链接等其他条目归为 `Other`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Other,
}

/// 各协议后端共用的错误类型。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemoteError {
    NameResolution,
    TimedOut,
    NoRoute,
    Refused,
    AuthenticationFailed,
    /// 身份有效，但没有权限访问这个导出、共享或目录。
    AccessDenied,
    NotFound,
    NotADirectory,
}

impl RemoteError {
    /// 换一个地址重试，是否可能成功。
    pub fn allows_next_address(&self) -> bool {
        let _ = self;
        todo!("P5.2: decide whether another address may help")
    }
}

impl fmt::Display for RemoteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NameResolution => "Host name could not be resolved",
            Self::TimedOut => "Connection timed out",
            Self::NoRoute => "No route to host",
            Self::Refused => "Connection refused",
            Self::AuthenticationFailed => "Authentication failed",
            Self::AccessDenied => "Access denied",
            Self::NotFound => "Remote path not found",
            Self::NotADirectory => "Remote path is not a directory",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RemoteError {}

/// 可复用的协议后端。
pub trait RemoteBackend {
    /// 对单个地址尝试建立会话，地址回退由路由层负责。
    fn connect(
        &self,
        host: &str,
        connection: &ProtocolConnection,
    ) -> Result<Box<dyn RemoteSession>, RemoteError>;
}

/// 已建立的会话。每次操作独占会话状态。
pub trait RemoteSession {
    /// `path` 从远程连接根目录算起，以 `/` 开头。条目可包含 `.` 和 `..`，顺序由调用方整理。
    /// 路径缺失返回 `NotFound`，路径指向文件时返回 `NotADirectory`。
    fn list_directory(&mut self, path: &str) -> Result<Vec<DirectoryEntry>, RemoteError>;
}

pub struct Connected {
    pub address_id: String,
    pub session: Box<dyn RemoteSession>,
}

// 会话无需实现 Debug，错误断言仍可使用 expect_err。
impl fmt::Debug for Connected {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Connected")
            .field("address_id", &self.address_id)
            .finish_non_exhaustive()
    }
}

/// 按地址优先级依次连接，返回第一个连上的会话。
pub fn connect_by_priority<B: RemoteBackend + ?Sized>(
    backend: &B,
    device: &Device,
    connection: &ProtocolConnection,
) -> Result<Connected, ConnectError> {
    let _ = (backend, device, connection);
    todo!("P5.3: try addresses in priority order")
}

/// 按连接协议选择对应的后端。
pub struct Backends<N, S> {
    pub nfs: N,
    pub smb: S,
}

impl<N: RemoteBackend, S: RemoteBackend> Backends<N, S> {
    /// 只使用连接所属协议的后端。
    pub fn connect(&self, device: &Device, connection_id: &str) -> Result<Connected, ConnectError> {
        let _ = (device, connection_id);
        todo!("P5.4: pick the backend for the connection's protocol")
    }
}

/// 读取一个目录，整理成稳定的显示顺序。
pub fn list_directory_sorted(
    session: &mut dyn RemoteSession,
    path: &str,
) -> Result<Vec<DirectoryEntry>, RemoteError> {
    let _ = (session, path);
    todo!("P5.5: list one directory in display order")
}

#[derive(Debug)]
pub enum ConnectError {
    ConnectionNotFound(String),
    NoAddresses,
    /// 遇到不可回退的错误后立即停止。
    Rejected {
        address_id: String,
        source: RemoteError,
    },
    /// 每个地址都连不上；按尝试顺序记录 (地址 ID, 原因)。
    AllUnreachable(Vec<(String, RemoteError)>),
}

impl fmt::Display for ConnectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionNotFound(id) => write!(formatter, "Connection not found: {id}"),
            Self::NoAddresses => write!(formatter, "Device has no addresses"),
            Self::Rejected { address_id, source } => {
                write!(
                    formatter,
                    "Connection via address {address_id} failed: {source}"
                )
            }
            Self::AllUnreachable(failures) => {
                write!(formatter, "All addresses are unreachable:")?;
                for (index, (address_id, error)) in failures.iter().enumerate() {
                    let separator = if index == 0 { " " } else { ", " };
                    write!(formatter, "{separator}{address_id} ({error})")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ConnectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Rejected { source, .. } => Some(source),
            _ => None,
        }
    }
}
