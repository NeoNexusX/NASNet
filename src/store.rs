//! config.toml 管理软件设置与设备引用；设备文件独立读写。库内不输出日志。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{fmt, io};

use crate::{
    config::{AppConfig, ConfigIoError, DeviceReference, decode_app_config},
    model::Device,
};

/// 只持有主配置路径，不代表网络连接，也不在构造时读取设备。
pub struct StoreConnection {
    config_path: PathBuf,
}

impl StoreConnection {
    /// 使用目录中的 config.toml。
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            config_path: directory.into().join("config.toml"),
        }
    }

    /// 启动入口已选定明确的主文件；不隐式读取，也不改用进程工作目录。
    pub fn from_config_path(config_path: impl Into<PathBuf>) -> Self {
        Self {
            config_path: config_path.into(),
        }
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// 显式指定的配置必须存在；保留路径与来源，不把损坏配置视为默认值。
    pub fn load_app_required(&self) -> Result<AppConfig, StoreError> {
        let text = std::fs::read_to_string(&self.config_path)
            .map_err(|error| StoreError::io("read configuration", &self.config_path, error))?;
        decode_app_config(&text).map_err(|error| StoreError::config(&self.config_path, error))
    }

    /// 默认位置的主文件可以不存在：只有 NotFound 换成内置默认值，其他错误照常返回。
    pub fn load_app(&self) -> Result<AppConfig, StoreError> {
        match self.load_app_required() {
            Err(StoreError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
                Ok(AppConfig::default())
            }
            result => result,
        }
    }

    pub fn resolve_device_path(&self, reference: &DeviceReference) -> Result<PathBuf, StoreError> {
        // TODO P4.4：先用 is_managed_device_path 检查路径形式，再相对 config.toml 所在目录组合。
        // 不相对进程 cwd，也不要求文件已经存在。
        let _ = reference;
        todo!("P4.4: resolve a device path relative to config.toml")
    }

    pub fn load_device(&self, reference: &DeviceReference) -> Result<Device, StoreError> {
        // TODO P4.5：解析路径 → 只读这一个设备文件 → decode_device_toml → 核对文件内 ID 与引用一致。
        // 已登记的设备文件缺失必须报错，不能当成空设备。
        let _ = reference;
        todo!("P4.5: load and validate one referenced device")
    }

    pub fn save_app(&mut self, config: &AppConfig) -> Result<(), StoreError> {
        // TODO P4.7：encode_app_config 成功后再 write_atomic 主配置；不读取或重写任何设备文件。
        let _ = config;
        todo!("P4.7: save config.toml without rewriting devices")
    }

    pub fn save_device(
        &mut self,
        reference: &DeviceReference,
        device: &Device,
    ) -> Result<(), StoreError> {
        // TODO P4.8：解析路径并核对 ID，再 encode_device_toml，最后 write_atomic 这一个设备文件。
        // 不重写 config.toml 或其他设备文件。
        let _ = (reference, device);
        todo!("P4.8: save only the selected device file")
    }

    pub fn register_device(
        &mut self,
        config: &mut AppConfig,
        reference: DeviceReference,
        device: &Device,
    ) -> Result<(), StoreError> {
        // TODO P4.9：在候选副本中追加引用并校验；新路径上已有文件时拒绝覆盖。
        // 先保存设备文件，再保存候选主配置，都成功后才替换内存中的 config。
        // 主配置保存失败可能留下未引用的设备文件：返回错误即可，不声称多文件回滚。
        let _ = (config, reference, device);
        todo!("P4.9: register a device, device file first, then config.toml")
    }

    pub fn remove_device_reference(
        &mut self,
        config: &mut AppConfig,
        id: &str,
    ) -> Result<bool, StoreError> {
        // TODO P4.10：只从候选副本移除引用并保存，成功后替换内存中的 config；不存在返回 false。
        // 设备文件保留，不在这里删除。
        let _ = (config, id);
        todo!("P4.10: remove a device reference without deleting its file")
    }
}

/// 每次只替换一个目标文件，不提供多文件事务。
pub fn write_atomic(destination: &Path, contents: &[u8]) -> Result<(), StoreError> {
    // TODO P4.6：创建父目录 → 同目录 create_new 临时文件 → write_all → sync_all → 关闭 → rename。
    // 任一步失败时尽力删除本次的临时文件并返回原始错误；不先删除目标，不删除别人的文件。
    // 未实现目录 fsync，不承诺完整的掉电持久性。
    let _ = (temporary_path(destination), contents);
    todo!("P4.6: atomically replace one file")
}

/// 同一目录下的唯一临时文件名：进程号区分进程，序号区分同一进程内的多次调用。
fn temporary_path(destination: &Path) -> PathBuf {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let name = destination
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    destination.with_file_name(format!(".{name}.tmp-{}-{sequence}", std::process::id()))
}

/// 存储错误：I/O 失败和配置内容错误都带上文件路径，并保留下层来源。
#[derive(Debug)]
pub enum StoreError {
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Config {
        path: PathBuf,
        source: ConfigIoError,
    },
    InvalidConfigPath,
}

impl StoreError {
    pub fn io(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            operation,
            path: path.into(),
            source,
        }
    }

    pub fn config(path: impl Into<PathBuf>, source: ConfigIoError) -> Self {
        Self::Config {
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
            Self::Config { path, source } => write!(
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
            Self::Config { source, .. } => Some(source),
            Self::InvalidConfigPath => None,
        }
    }
}
