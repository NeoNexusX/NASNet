//! 主配置与设备文件的存储。

use std::{
    fmt,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    config::{
        AppConfig, ConfigError, ConfigIoError, DeviceReference, decode_app_config,
        decode_device_toml, encode_app_config, encode_device_toml, is_managed_device_path,
    },
    model::Device,
};

/// 以主配置路径为基准访问配置文件。
pub struct StoreConnection {
    config_path: PathBuf,
}

impl StoreConnection {
    /// 使用目录中的 config.toml。
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self::from_config_path(directory.into().join("config.toml"))
    }

    pub fn from_config_path(config_path: impl Into<PathBuf>) -> Self {
        Self {
            config_path: config_path.into(),
        }
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// 读取主配置。`required` 为 false 时允许初始化缺失的文件。
    pub fn load_app(&self, required: bool) -> Result<AppConfig, StoreError> {
        let text = match fs::read_to_string(&self.config_path) {
            Err(error) if !required && error.kind() == io::ErrorKind::NotFound => {
                return self.create_default_app();
            }
            result => result
                .map_err(|error| StoreError::io("read configuration", &self.config_path, error))?,
        };
        decode_app_config(&text).map_err(|error| StoreError::config(&self.config_path, error))
    }

    fn create_default_app(&self) -> Result<AppConfig, StoreError> {
        let config = AppConfig::default();
        let text = encode_app_config(&config)
            .map_err(|error| StoreError::config(&self.config_path, error))?;
        let temp = stage_file(&self.config_path, text.as_bytes())?;

        // 只发布完整文件。若另一进程先创建了目标，读取它写入的配置。
        let result = fs::hard_link(&temp, &self.config_path);
        let _ = fs::remove_file(&temp);
        match result {
            Ok(()) => Ok(config),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => self.load_app(true),
            Err(error) => Err(StoreError::io(
                "create configuration",
                &self.config_path,
                error,
            )),
        }
    }

    pub fn resolve_device_path(&self, reference: &DeviceReference) -> Result<PathBuf, StoreError> {
        if !is_managed_device_path(&reference.path) {
            return Err(StoreError::config(
                &self.config_path,
                ConfigError::InvalidDeviceReference.into(),
            ));
        }
        let device_path = self
            .config_path
            .parent()
            .ok_or(StoreError::InvalidConfigPath)?
            .join(&reference.path);

        Ok(device_path)
    }

    pub fn load_device(&self, reference: &DeviceReference) -> Result<Device, StoreError> {
        let path = self.resolve_device_path(reference)?;

        let device_data = fs::read_to_string(&path)
            .map_err(|error| StoreError::io("read device", &path, error))?;

        let device =
            decode_device_toml(&device_data).map_err(|error| StoreError::config(&path, error))?;

        if device.id() != reference.id {
            return Err(StoreError::config(
                &path,
                ConfigError::DeviceIdMismatch.into(),
            ));
        }
        Ok(device)
    }

    pub fn save_app(&mut self, config: &AppConfig) -> Result<(), StoreError> {
        let text = encode_app_config(config)
            .map_err(|error| StoreError::config(&self.config_path, error))?;
        write_atomic(&self.config_path, text.as_bytes())
    }

    pub fn save_device(
        &mut self,
        reference: &DeviceReference,
        device: &Device,
    ) -> Result<(), StoreError> {
        let path = self.resolve_device_path(reference)?;
        save_device_at(&path, &reference.id, device)
    }

    /// 先保存设备，再发布引用，避免主配置引用缺失文件。
    pub fn register_device(
        &mut self,
        config: &mut AppConfig,
        reference: DeviceReference,
        device: &Device,
    ) -> Result<(), StoreError> {
        let mut new_config = config.clone();

        new_config.devices.push(reference);

        let reference = new_config.devices.last().expect("刚添加了一条设备引用");

        new_config
            .validate()
            .map_err(|error| StoreError::config(&self.config_path, error))?;

        let destination = self.resolve_device_path(reference)?;

        if destination
            .try_exists()
            .map_err(|err| StoreError::io("check device path", &destination, err))?
        {
            return Err(StoreError::io(
                "register device",
                &destination,
                io::ErrorKind::AlreadyExists.into(),
            ));
        }

        save_device_at(&destination, &reference.id, device)?;
        self.save_app(&new_config)?;

        *config = new_config;

        Ok(())
    }

    /// 移除清单中的引用，保留设备文件。
    pub fn remove_device_reference(
        &mut self,
        config: &mut AppConfig,
        id: &str,
    ) -> Result<bool, StoreError> {
        let Some(index) = config.devices.iter().position(|device| device.id == id) else {
            return Ok(false);
        };
        let mut new_config = config.clone();
        new_config.devices.remove(index);

        self.save_app(&new_config)?;

        *config = new_config;
        Ok(true)
    }
}

fn save_device_at(path: &Path, expected_id: &str, device: &Device) -> Result<(), StoreError> {
    if device.id() != expected_id {
        return Err(StoreError::config(
            path,
            ConfigError::DeviceIdMismatch.into(),
        ));
    }
    let text = encode_device_toml(device).map_err(|error| StoreError::config(path, error))?;
    write_atomic(path, text.as_bytes())
}

/// 原子替换单个文件。
pub fn write_atomic(destination: &Path, contents: &[u8]) -> Result<(), StoreError> {
    let temp = stage_file(destination, contents)?;
    if let Err(error) = fs::rename(&temp, destination) {
        let _ = fs::remove_file(&temp);
        return Err(StoreError::io("replace file", destination, error));
    }
    Ok(())
}

fn stage_file(destination: &Path, contents: &[u8]) -> Result<PathBuf, StoreError> {
    let parent = destination.parent().ok_or(StoreError::InvalidConfigPath)?;
    fs::create_dir_all(parent)
        .map_err(|error| StoreError::io("create directory", parent, error))?;

    let temp = temporary_path(destination);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| StoreError::io("create temporary file", &temp, error))?;

    let result = file
        .write_all(contents)
        .map_err(|error| StoreError::io("write temporary file", &temp, error))
        .and_then(|()| {
            file.sync_all()
                .map_err(|error| StoreError::io("sync temporary file", &temp, error))
        });
    drop(file);
    if let Err(error) = result {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    Ok(temp)
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
