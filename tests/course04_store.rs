mod common;

use common::{APP_CONFIG, BACKUP_DEVICE, HOME_DEVICE, TestResult};
use nasnet::{
    config::{
        AppConfig, ConfigError, ConfigIoError, DeviceReference, decode_app_config,
        encode_app_config,
    },
    model::Device,
    store::{StoreConnection, StoreError, write_atomic},
};
use std::{
    error::Error,
    fs,
    io::{self, ErrorKind},
    path::Path,
};

#[test]
fn missing_main_configuration_is_created_from_defaults() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = StoreConnection::new(directory.path());
    let config = store.load_app(false)?;
    assert!(config.devices.is_empty());
    assert_eq!(config.logging.console_level, "info");
    assert_eq!(config.logging.file_level, "trace");
    assert_eq!(
        fs::read_to_string(store.config_path())?,
        encode_app_config(&AppConfig::default())?
    );
    assert_eq!(file_names(directory.path())?, ["config.toml"]);
    Ok(())
}

#[test]
fn read_only_load_reports_a_missing_configuration() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("settings.toml");
    let store = StoreConnection::from_config_path(&path);
    assert!(
        matches!(store.load_app(true), Err(StoreError::Io { path: failed_path, source, .. }) if failed_path == path && source.kind() == ErrorKind::NotFound)
    );
    assert!(!path.exists());
    Ok(())
}

#[test]
fn concurrent_initialization_leaves_one_complete_configuration() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = StoreConnection::new(directory.path());
    let barrier = std::sync::Barrier::new(4);
    std::thread::scope(|scope| -> TestResult {
        let workers: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    store.load_app(false)
                })
            })
            .collect();
        for worker in workers {
            assert_eq!(worker.join().unwrap()?.software.language, "en-US");
        }
        Ok(())
    })?;
    let config = decode_app_config(&fs::read_to_string(store.config_path())?)?;
    assert_eq!(config.software.language, "en-US");
    assert_eq!(file_names(directory.path())?, ["config.toml"]);
    Ok(())
}

#[test]
fn reads_only_the_main_file_without_loading_referenced_devices() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("settings.toml");
    fs::write(&path, APP_CONFIG)?;
    let store = StoreConnection::from_config_path(&path);
    let config = store.load_app(true)?;
    assert_eq!(config.devices.len(), 2);
    assert!(!directory.path().join("devices").exists());
    assert!(!directory.path().join("logs").exists());
    assert_eq!(fs::read(&path)?, APP_CONFIG.as_bytes());
    Ok(())
}

#[test]
fn corrupt_main_file_is_not_replaced_with_defaults() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("config.toml");
    let store = StoreConnection::new(directory.path());
    for contents in ["", "schema_version = ["] {
        fs::write(&path, contents)?;
        assert!(
            matches!(store.load_app(false), Err(StoreError::Config { path: failed_path, source: ConfigIoError::TomlDecode(_) }) if failed_path == path)
        );
        assert_eq!(fs::read_to_string(&path)?, contents);
    }
    Ok(())
}

#[test]
fn unsupported_version_keeps_configuration_file_context() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("config.toml");
    fs::write(
        &path,
        APP_CONFIG.replace("schema_version = 1", "schema_version = 99"),
    )?;
    let store = StoreConnection::new(directory.path());
    assert!(
        matches!(store.load_app(false), Err(StoreError::Config { path: failed_path, source: ConfigIoError::UnsupportedVersion(99) }) if failed_path == path)
    );
    Ok(())
}

#[test]
fn directory_in_place_of_main_file_remains_an_io_error() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("config.toml");
    fs::create_dir(&path)?;
    let store = StoreConnection::new(directory.path());
    assert!(
        matches!(store.load_app(false), Err(StoreError::Io { source, .. }) if source.kind() != ErrorKind::NotFound)
    );
    Ok(())
}

#[test]
fn bad_device_files_do_not_affect_main_configuration_loading() -> TestResult {
    let directory = tempfile::tempdir()?;
    fs::write(directory.path().join("config.toml"), APP_CONFIG)?;
    fs::create_dir(directory.path().join("devices"))?;
    let device_path = directory.path().join("devices/home.toml");
    let contents = b"not a valid device document";
    fs::write(&device_path, contents)?;
    let store = StoreConnection::new(directory.path());
    assert_eq!(store.load_app(false)?.devices.len(), 2);
    assert_eq!(fs::read(&device_path)?, contents);
    Ok(())
}

// P4.4–P4.10 共用的夹具

/// 临时目录里放好示例主配置和两个设备文件，布局与 examples/configuration 相同。
/// TempDir 要一起返回：它被 drop 时，整个目录会被删除。
fn example_store() -> Result<(tempfile::TempDir, StoreConnection), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(directory.path().join("config.toml"), APP_CONFIG)?;
    fs::create_dir(directory.path().join("devices"))?;
    fs::write(directory.path().join("devices/home.toml"), HOME_DEVICE)?;
    fs::write(directory.path().join("devices/backup.toml"), BACKUP_DEVICE)?;
    let store = StoreConnection::new(directory.path());
    Ok((directory, store))
}

fn reference(id: &str, path: &str) -> DeviceReference {
    DeviceReference {
        id: id.into(),
        path: path.into(),
    }
}

/// 设备清单里的 ID，保持清单顺序。
fn reference_ids(config: &AppConfig) -> Vec<&str> {
    config.devices.iter().map(|item| item.id.as_str()).collect()
}

/// 目录里的文件名，排好序；用来确认没有残留的临时文件。
fn file_names(directory: &Path) -> io::Result<Vec<String>> {
    let mut names = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<io::Result<Vec<_>>>()?;
    names.sort();
    Ok(names)
}

// P4.4 resolve_device_path

#[test]
fn device_paths_are_resolved_from_the_configuration_directory() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = StoreConnection::new(directory.path());
    let path = store.resolve_device_path(&reference("nas-home", "devices/home.toml"))?;
    assert_eq!(path, directory.path().join("devices/home.toml"));
    // 只算路径：文件不存在也能解析，也不会创建任何文件或目录。
    assert_eq!(fs::read_dir(directory.path())?.count(), 0);
    Ok(())
}

#[test]
fn unmanaged_device_paths_are_rejected() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = StoreConnection::new(directory.path());
    for path in ["../a.toml", "/a.toml", "a.txt"] {
        assert!(
            matches!(
                store.resolve_device_path(&reference("nas", path)),
                Err(StoreError::Config {
                    source: ConfigIoError::Validation(ConfigError::InvalidDeviceReference),
                    ..
                })
            ),
            "path: {path}"
        );
    }
    Ok(())
}

// P4.5 load_device

#[test]
fn loads_the_referenced_device() -> TestResult {
    let (_directory, store) = example_store()?;
    let config = store.load_app(true)?;
    let device = store.load_device(&config.devices[0])?;
    assert_eq!((device.id(), device.name()), ("nas-home", "Home NAS"));
    Ok(())
}

#[test]
fn a_corrupt_device_file_does_not_affect_other_devices() -> TestResult {
    let (directory, store) = example_store()?;
    fs::write(
        directory.path().join("devices/backup.toml"),
        "not a valid device document",
    )?;
    let device = store.load_device(&reference("nas-home", "devices/home.toml"))?;
    assert_eq!(device.id(), "nas-home");
    Ok(())
}

#[test]
fn a_missing_device_file_is_an_io_error_for_that_file() -> TestResult {
    let (directory, store) = example_store()?;
    let path = directory.path().join("devices/home.toml");
    fs::remove_file(&path)?;
    assert!(
        matches!(store.load_device(&reference("nas-home", "devices/home.toml")), Err(StoreError::Io { path: failed_path, source, .. }) if failed_path == path && source.kind() == ErrorKind::NotFound)
    );
    Ok(())
}

#[test]
fn a_reference_to_another_devices_file_is_an_id_mismatch() -> TestResult {
    let (directory, store) = example_store()?;
    let path = directory.path().join("devices/backup.toml");
    assert!(
        matches!(store.load_device(&reference("nas-home", "devices/backup.toml")), Err(StoreError::Config { path: failed_path, source: ConfigIoError::Validation(ConfigError::DeviceIdMismatch) }) if failed_path == path)
    );
    Ok(())
}

// P4.6 write_atomic

#[test]
fn atomic_writes_create_parents_and_replace_contents_without_leftovers() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("nested/devices/home.toml");
    write_atomic(&path, b"first")?;
    assert_eq!(fs::read(&path)?, b"first");
    write_atomic(&path, b"second")?;
    assert_eq!(fs::read(&path)?, b"second");
    assert_eq!(
        file_names(&directory.path().join("nested/devices"))?,
        ["home.toml"]
    );
    Ok(())
}

#[test]
fn an_atomic_write_onto_a_directory_fails_and_cleans_up() -> TestResult {
    let directory = tempfile::tempdir()?;
    let target = directory.path().join("config.toml");
    fs::create_dir(&target)?;
    fs::write(target.join("keep.txt"), "KEEP")?;
    assert!(matches!(
        write_atomic(&target, b"new"),
        Err(StoreError::Io { .. })
    ));
    assert_eq!(fs::read_to_string(target.join("keep.txt"))?, "KEEP");
    assert_eq!(file_names(directory.path())?, ["config.toml"]);
    Ok(())
}

// P4.7 save_app

#[test]
fn saved_main_configuration_reads_back_and_device_files_stay_unchanged() -> TestResult {
    let (directory, mut store) = example_store()?;
    let mut config = store.load_app(true)?;
    config.software.language = "zh-CN".into();
    store.save_app(&config)?;
    let saved = store.load_app(true)?;
    assert_eq!(saved.software.language, "zh-CN");
    assert_eq!(saved.logging.file_path, config.logging.file_path);
    assert_eq!(reference_ids(&saved), ["nas-home", "nas-backup"]);
    assert_eq!(
        fs::read(directory.path().join("devices/home.toml"))?,
        HOME_DEVICE.as_bytes()
    );
    assert_eq!(
        fs::read(directory.path().join("devices/backup.toml"))?,
        BACKUP_DEVICE.as_bytes()
    );
    Ok(())
}

#[test]
fn invalid_main_configuration_is_not_saved() -> TestResult {
    let (directory, mut store) = example_store()?;
    let mut config = store.load_app(true)?;
    config.logging.console_level = "verbose".into();
    let path = directory.path().join("config.toml");
    assert!(
        matches!(store.save_app(&config), Err(StoreError::Config { path: failed_path, source: ConfigIoError::Validation(ConfigError::InvalidLoggingSettings) }) if failed_path == path)
    );
    assert_eq!(fs::read(&path)?, APP_CONFIG.as_bytes());
    Ok(())
}

// P4.8 save_device

#[test]
fn saved_device_reads_back_and_other_files_stay_unchanged() -> TestResult {
    let (directory, mut store) = example_store()?;
    let home = reference("nas-home", "devices/home.toml");
    let mut device = store.load_device(&home)?;
    device.rename("Renamed NAS".into());
    store.save_device(&home, &device)?;
    assert_eq!(store.load_device(&home)?.name(), "Renamed NAS");
    assert_eq!(
        fs::read(directory.path().join("config.toml"))?,
        APP_CONFIG.as_bytes()
    );
    assert_eq!(
        fs::read(directory.path().join("devices/backup.toml"))?,
        BACKUP_DEVICE.as_bytes()
    );
    Ok(())
}

#[test]
fn a_device_id_mismatch_creates_no_file() -> TestResult {
    let (directory, mut store) = example_store()?;
    let device = store.load_device(&reference("nas-home", "devices/home.toml"))?;
    let path = directory.path().join("devices/other.toml");
    assert!(
        matches!(store.save_device(&reference("nas-other", "devices/other.toml"), &device), Err(StoreError::Config { path: failed_path, source: ConfigIoError::Validation(ConfigError::DeviceIdMismatch) }) if failed_path == path)
    );
    assert!(!path.exists());
    Ok(())
}

// P4.9 register_device

/// 登记用的新设备；没有地址和连接的设备也是合法的。
fn office_device() -> Device {
    Device::new("nas-office".into(), "Office NAS".into())
}

#[test]
fn registering_writes_the_device_file_and_the_reference() -> TestResult {
    let (_directory, mut store) = example_store()?;
    let mut config = store.load_app(true)?;
    let office = reference("nas-office", "devices/office.toml");
    store.register_device(&mut config, office.clone(), &office_device())?;
    assert_eq!(
        reference_ids(&config),
        ["nas-home", "nas-backup", "nas-office"]
    );
    assert_eq!(
        reference_ids(&store.load_app(true)?),
        ["nas-home", "nas-backup", "nas-office"]
    );
    assert_eq!(store.load_device(&office)?.name(), "Office NAS");
    Ok(())
}

#[test]
fn duplicate_ids_are_rejected_before_any_file_is_written() -> TestResult {
    let (directory, mut store) = example_store()?;
    let mut config = store.load_app(true)?;
    let duplicate = Device::new("nas-home".into(), "Another NAS".into());
    assert!(
        matches!(store.register_device(&mut config, reference("nas-home", "devices/office.toml"), &duplicate), Err(StoreError::Config { source: ConfigIoError::Validation(ConfigError::DuplicateDeviceId(id)), .. }) if id == "nas-home")
    );
    assert_eq!(reference_ids(&config), ["nas-home", "nas-backup"]);
    assert!(!directory.path().join("devices/office.toml").exists());
    assert_eq!(
        fs::read(directory.path().join("config.toml"))?,
        APP_CONFIG.as_bytes()
    );
    Ok(())
}

#[test]
fn an_unregistered_file_at_the_new_path_is_not_overwritten() -> TestResult {
    let (directory, mut store) = example_store()?;
    let mut config = store.load_app(true)?;
    let path = directory.path().join("devices/office.toml");
    fs::write(&path, "USER_FILE")?;
    assert!(
        matches!(store.register_device(&mut config, reference("nas-office", "devices/office.toml"), &office_device()), Err(StoreError::Io { source, .. }) if source.kind() == ErrorKind::AlreadyExists)
    );
    assert_eq!(fs::read_to_string(&path)?, "USER_FILE");
    assert_eq!(reference_ids(&config), ["nas-home", "nas-backup"]);
    assert_eq!(
        fs::read(directory.path().join("config.toml"))?,
        APP_CONFIG.as_bytes()
    );
    Ok(())
}

#[test]
fn a_failed_main_configuration_write_leaves_an_unreferenced_device_file() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut store = StoreConnection::new(directory.path());
    // 主配置的位置是一个目录：设备文件能写成，替换主配置时失败。
    fs::create_dir(store.config_path())?;
    let mut config = decode_app_config(APP_CONFIG)?;
    let office = reference("nas-office", "devices/office.toml");
    assert!(matches!(
        store.register_device(&mut config, office.clone(), &office_device()),
        Err(StoreError::Io { .. })
    ));
    assert_eq!(reference_ids(&config), ["nas-home", "nas-backup"]);
    // 先写设备、后发布引用：设备文件已经写好，但没有主配置引用它。
    assert_eq!(store.load_device(&office)?.id(), "nas-office");
    assert!(store.config_path().is_dir());
    Ok(())
}

// P4.10 remove_device_reference

#[test]
fn removing_a_reference_keeps_the_device_file() -> TestResult {
    let (directory, mut store) = example_store()?;
    let mut config = store.load_app(true)?;
    assert!(store.remove_device_reference(&mut config, "nas-backup")?);
    assert_eq!(reference_ids(&config), ["nas-home"]);
    assert_eq!(reference_ids(&store.load_app(true)?), ["nas-home"]);
    assert_eq!(
        fs::read(directory.path().join("devices/backup.toml"))?,
        BACKUP_DEVICE.as_bytes()
    );
    Ok(())
}

#[test]
fn removing_an_unknown_id_writes_nothing() -> TestResult {
    let (directory, mut store) = example_store()?;
    let mut config = store.load_app(true)?;
    assert!(!store.remove_device_reference(&mut config, "nas-none")?);
    assert_eq!(reference_ids(&config), ["nas-home", "nas-backup"]);
    assert_eq!(
        fs::read(directory.path().join("config.toml"))?,
        APP_CONFIG.as_bytes()
    );
    Ok(())
}

#[test]
fn a_failed_removal_keeps_the_reference_in_memory() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut store = StoreConnection::new(directory.path());
    // 和 P4.9 一样，用目录占住主配置的位置，让保存失败。
    fs::create_dir(store.config_path())?;
    let mut config = decode_app_config(APP_CONFIG)?;
    assert!(matches!(
        store.remove_device_reference(&mut config, "nas-backup"),
        Err(StoreError::Io { .. })
    ));
    assert_eq!(reference_ids(&config), ["nas-home", "nas-backup"]);
    Ok(())
}
