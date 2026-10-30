mod common;

use common::{APP_CONFIG, TestResult};
use nasnet::{
    config::ConfigIoError,
    store::{StoreConnection, StoreError},
};
use std::{fs, io::ErrorKind};

#[test]
fn missing_default_config_uses_defaults_without_creating_files() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = StoreConnection::new(directory.path());
    let config = store.load_app()?;
    assert!(config.devices.is_empty());
    assert_eq!(config.logging.console_level, "info");
    assert_eq!(config.logging.file_level, "trace");
    assert_eq!(fs::read_dir(directory.path())?.count(), 0);
    Ok(())
}

#[test]
fn missing_explicit_config_preserves_io_source_and_path() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("settings.toml");
    let store = StoreConnection::from_config_path(&path);
    assert!(
        matches!(store.load_app_required(), Err(StoreError::Io { path: failed_path, source, .. }) if failed_path == path && source.kind() == ErrorKind::NotFound)
    );
    assert!(!path.exists());
    Ok(())
}

#[test]
fn reads_only_the_main_file_without_loading_referenced_devices() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("settings.toml");
    fs::write(&path, APP_CONFIG)?;
    let store = StoreConnection::from_config_path(&path);
    let config = store.load_app_required()?;
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
            matches!(store.load_app(), Err(StoreError::Config { path: failed_path, source: ConfigIoError::TomlDecode(_) }) if failed_path == path)
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
        matches!(store.load_app(), Err(StoreError::Config { path: failed_path, source: ConfigIoError::UnsupportedVersion(99) }) if failed_path == path)
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
        matches!(store.load_app(), Err(StoreError::Io { source, .. }) if source.kind() != ErrorKind::NotFound)
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
    assert_eq!(store.load_app()?.devices.len(), 2);
    assert_eq!(fs::read(&device_path)?, contents);
    Ok(())
}
