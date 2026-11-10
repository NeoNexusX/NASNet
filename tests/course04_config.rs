mod common;

use common::{APP_CONFIG, BACKUP_DEVICE, HOME_DEVICE, TestResult};
use nasnet::config::{
    AppConfig, ConfigError, ConfigIoError, DeviceReference, SCHEMA_VERSION, decode_app_config,
    decode_device_toml, encode_app_config, encode_device_toml,
};
use nasnet::model::{
    Address, ConnectionConfig, Device, ModelError, ProtocolConnection, SmbAuth, SmbConfig,
    SmbDialect,
};
use std::{error::Error, path::Path};

#[test]
fn default_config_is_valid() -> TestResult {
    let config = AppConfig::default();
    config.validate()?;
    assert_eq!(config.schema_version, SCHEMA_VERSION);
    assert_eq!(config.software.language, "en-US");
    assert_eq!(config.logging.console_level, "info");
    assert_eq!(config.logging.file_level, "trace");
    assert_eq!(config.logging.file_path.to_str(), Some("logs/nasnet.log"));
    assert!(config.devices.is_empty());
    Ok(())
}

#[test]
fn decodes_example_settings_and_preserves_reference_order() -> TestResult {
    let config = decode_app_config(APP_CONFIG)?;
    assert_eq!(config.software.language, "en-US");
    assert_eq!(
        config
            .devices
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        ["nas-home", "nas-backup"]
    );
    assert_eq!(config.devices[0].path.to_str(), Some("devices/home.toml"));
    Ok(())
}

#[test]
fn accepts_each_supported_log_level() -> TestResult {
    for level in ["trace", "debug", "info", "warn", "error", "off"] {
        let mut config = AppConfig::default();
        config.logging.console_level = level.into();
        config.logging.file_level = level.into();
        config.validate()?;
    }
    Ok(())
}

#[test]
fn rejects_unknown_console_level() {
    let mut config = AppConfig::default();
    config.logging.console_level = "verbose".into();
    assert!(matches!(
        config.validate(),
        Err(ConfigIoError::Validation(
            ConfigError::InvalidLoggingSettings
        ))
    ));
}

#[test]
fn rejects_unknown_file_level() {
    let mut config = AppConfig::default();
    config.logging.file_level = "verbose".into();
    assert!(matches!(
        config.validate(),
        Err(ConfigIoError::Validation(
            ConfigError::InvalidLoggingSettings
        ))
    ));
}

#[test]
fn rejects_log_paths_without_a_file_name() {
    for path in ["", ".", ".."] {
        let mut config = AppConfig::default();
        config.logging.file_path = path.into();
        assert!(
            matches!(
                config.validate(),
                Err(ConfigIoError::Validation(
                    ConfigError::InvalidLoggingSettings
                ))
            ),
            "path: {path:?}"
        );
    }
}

#[cfg(windows)]
#[test]
fn rejects_windows_drive_relative_and_root_relative_log_paths() {
    for path in [r"D:logs\nasnet.log", r"\logs\nasnet.log"] {
        let mut config = AppConfig::default();
        config.logging.file_path = path.into();
        assert!(
            matches!(
                config.validate(),
                Err(ConfigIoError::Validation(
                    ConfigError::InvalidLoggingSettings
                ))
            ),
            "path: {path}"
        );
    }
}

#[test]
fn rejects_blank_language() {
    let mut config = AppConfig::default();
    config.software.language = " \t".into();
    assert!(matches!(
        config.validate(),
        Err(ConfigIoError::Validation(
            ConfigError::InvalidSoftwareSettings
        ))
    ));
}

#[test]
fn rejects_unsupported_version_before_other_validation() {
    let mut config = AppConfig {
        schema_version: 99,
        ..AppConfig::default()
    };
    config.software.language.clear();
    assert!(matches!(
        config.validate(),
        Err(ConfigIoError::UnsupportedVersion(99))
    ));
}

#[test]
fn rejects_blank_reference_id() {
    let mut config = AppConfig::default();
    config.devices.push(DeviceReference {
        id: " ".into(),
        path: "devices/a.toml".into(),
    });
    assert!(matches!(
        config.validate(),
        Err(ConfigIoError::Validation(
            ConfigError::InvalidDeviceReference
        ))
    ));
}

#[test]
fn rejects_unmanaged_device_paths() {
    for path in ["", "../a.toml", "./a.toml", "/a.toml", "devices/a.txt"] {
        let mut config = AppConfig::default();
        config.devices.push(DeviceReference {
            id: "nas".into(),
            path: path.into(),
        });
        assert!(
            matches!(
                config.validate(),
                Err(ConfigIoError::Validation(
                    ConfigError::InvalidDeviceReference
                ))
            ),
            "path: {path:?}"
        );
    }
}

#[test]
fn rejects_duplicate_reference_ids() -> TestResult {
    let mut config = decode_app_config(APP_CONFIG)?;
    config.devices[1].id = config.devices[0].id.clone();
    assert!(
        matches!(config.validate(), Err(ConfigIoError::Validation(ConfigError::DuplicateDeviceId(id))) if id == "nas-home")
    );
    Ok(())
}

#[test]
fn rejects_duplicate_reference_paths() -> TestResult {
    let mut config = decode_app_config(APP_CONFIG)?;
    config.devices[1].path = config.devices[0].path.clone();
    assert!(matches!(
        config.validate(),
        Err(ConfigIoError::Validation(ConfigError::DuplicateDevicePath))
    ));
    Ok(())
}

#[test]
fn decoding_revalidates_candidate_values() {
    let text = APP_CONFIG.replace("schema_version = 1", "schema_version = 99");
    assert!(matches!(
        decode_app_config(&text),
        Err(ConfigIoError::UnsupportedVersion(99))
    ));
    let text = APP_CONFIG.replace("console_level = \"info\"", "console_level = \"verbose\"");
    assert!(matches!(
        decode_app_config(&text),
        Err(ConfigIoError::Validation(
            ConfigError::InvalidLoggingSettings
        ))
    ));
}

#[test]
fn rejects_unknown_and_missing_required_fields() {
    let unknown = APP_CONFIG.replace(
        "language = \"en-US\"",
        "language = \"en-US\"\nunknown = true",
    );
    let missing = APP_CONFIG.replace("console_level = \"info\"", "");
    for text in [&unknown, &missing] {
        assert!(matches!(
            decode_app_config(text),
            Err(ConfigIoError::TomlDecode(_))
        ));
    }
}

#[test]
fn parse_errors_keep_the_source_but_hide_input_in_display() {
    let error = match decode_app_config("TEST_SECRET_SENTINEL = [") {
        Err(error) => error,
        Ok(_) => panic!("malformed TOML must be rejected"),
    };
    assert!(matches!(error, ConfigIoError::TomlDecode(_)));
    assert!(error.source().is_some());
    assert!(!error.to_string().contains("TEST_SECRET_SENTINEL"));
}

#[test]
fn snapshot_failure_keeps_its_model_source_through_configuration_and_storage() {
    use nasnet::{config::DeviceData, store::StoreError};

    let candidate = DeviceData {
        id: "nas".into(),
        name: "Test NAS".into(),
        addresses: vec![Address {
            id: "home".into(),
            host: "https://TEST_SECRET_SENTINEL".into(),
            priority: 0,
        }],
        connections: Vec::new(),
    };
    let error = Device::try_from(candidate)
        .err()
        .expect("a host with a scheme must fail");
    let error = StoreError::config("devices/home.toml", ConfigIoError::from(error));
    let config_error = error
        .source()
        .unwrap()
        .downcast_ref::<ConfigIoError>()
        .unwrap();
    assert!(matches!(
        config_error,
        ConfigIoError::ModelValidation(ModelError::HostContainsSchemeOrPath)
    ));
    assert!(matches!(
        config_error.source().unwrap().downcast_ref::<ModelError>(),
        Some(ModelError::HostContainsSchemeOrPath)
    ));
    assert!(error.to_string().contains("devices/home.toml"));
    assert!(!error.to_string().contains("TEST_SECRET_SENTINEL"));
}

// P4.1 encode_app_config

/// 设备引用的 (id, path)，保持清单顺序；DeviceReference 没有实现 PartialEq，先取出字段再比较。
fn references(config: &AppConfig) -> Vec<(&str, &Path)> {
    config
        .devices
        .iter()
        .map(|item| (item.id.as_str(), item.path.as_path()))
        .collect()
}

#[test]
fn encoded_main_configuration_decodes_to_the_same_settings() -> TestResult {
    let original = decode_app_config(APP_CONFIG)?;
    let decoded = decode_app_config(&encode_app_config(&original)?)?;
    assert_eq!(decoded.schema_version, original.schema_version);
    assert_eq!(decoded.software.language, original.software.language);
    assert_eq!(
        decoded.logging.console_level,
        original.logging.console_level
    );
    assert_eq!(decoded.logging.file_level, original.logging.file_level);
    assert_eq!(decoded.logging.file_path, original.logging.file_path);
    assert_eq!(references(&decoded), references(&original));
    Ok(())
}

#[test]
fn invalid_main_configuration_is_not_encoded() {
    let mut config = AppConfig::default();
    config.logging.console_level = "verbose".into();
    assert!(matches!(
        encode_app_config(&config),
        Err(ConfigIoError::Validation(
            ConfigError::InvalidLoggingSettings
        ))
    ));
}

// P4.2 encode_device_toml

/// 与 examples/configuration/devices/home.toml 内容相同的设备，用模型 API 逐项添加。
fn home_device() -> Result<Device, ModelError> {
    let mut device = Device::new("nas-home".into(), "Home NAS".into());
    device.try_add_address(Address {
        id: "lan".into(),
        host: "192.168.50.10".into(),
        priority: 1,
    })?;
    device.try_add_address(Address {
        id: "domain".into(),
        host: "nas.example.com".into(),
        priority: 2,
    })?;
    device.try_add_connection(ProtocolConnection {
        id: "smb-data".into(),
        config: ConnectionConfig::Smb(SmbConfig {
            share_name: "data".into(),
            base_path: Some("photos/2026".into()),
            port: Some(445),
            authentication: SmbAuth {
                username: "nas-user".into(),
                domain: None,
                password: " demo-only-password ".into(),
            },
            dialect: SmbDialect::Smb3,
        }),
    })?;
    Ok(device)
}

#[test]
fn encoded_device_starts_with_its_version_and_keeps_password_spaces() -> TestResult {
    let text = encode_device_toml(&home_device()?)?;
    assert!(text.starts_with("schema_version = 1\n"));
    assert!(text.contains("id = \"lan\"") && text.contains("id = \"domain\""));
    assert!(text.contains("password = \" demo-only-password \""));
    Ok(())
}

// P4.3 decode_device_toml

#[test]
fn decodes_the_home_device_file() -> TestResult {
    let device = decode_device_toml(HOME_DEVICE)?;
    assert_eq!((device.id(), device.name()), ("nas-home", "Home NAS"));
    let address_ids: Vec<&str> = device
        .addresses()
        .iter()
        .map(|address| address.id.as_str())
        .collect();
    assert_eq!(address_ids, ["lan", "domain"]);
    let ConnectionConfig::Smb(smb) = &device.connections()[0].config else {
        panic!("smb-data must be decoded as an SMB connection");
    };
    assert_eq!(smb.authentication.password, " demo-only-password ");
    Ok(())
}

#[test]
fn decodes_a_device_without_addresses_or_connections() -> TestResult {
    let device = decode_device_toml(BACKUP_DEVICE)?;
    assert_eq!((device.id(), device.name()), ("nas-backup", "Backup NAS"));
    assert!(device.addresses().is_empty());
    assert!(device.connections().is_empty());
    Ok(())
}

#[test]
fn encoded_device_decodes_to_the_same_device() -> TestResult {
    let text = encode_device_toml(&home_device()?)?;
    let decoded = decode_device_toml(&text)?;
    // Device 没有实现 PartialEq：把解码结果再编码一次，两段文本相同就说明保存的字段都没丢。
    assert_eq!(encode_device_toml(&decoded)?, text);
    Ok(())
}

#[test]
fn device_files_with_unknown_fields_or_protocols_are_rejected() {
    let unknown_field =
        HOME_DEVICE.replace("name = \"Home NAS\"", "name = \"Home NAS\"\nextra = true");
    let unknown_protocol = HOME_DEVICE.replace("protocol = \"smb\"", "protocol = \"ftp\"");
    for text in [&unknown_field, &unknown_protocol] {
        assert!(matches!(
            decode_device_toml(text),
            Err(ConfigIoError::TomlDecode(_))
        ));
    }
}

#[test]
fn device_files_with_other_schema_versions_are_rejected() {
    // 比当前新或旧都拒绝：程序不知道那种格式长什么样。
    for version in [99, 0] {
        let text =
            HOME_DEVICE.replace("schema_version = 1", &format!("schema_version = {version}"));
        assert!(
            matches!(decode_device_toml(&text), Err(ConfigIoError::UnsupportedVersion(found)) if found == version),
            "schema_version = {version}"
        );
    }
}

#[test]
fn invalid_device_values_are_rejected_by_model_validation() {
    let text = HOME_DEVICE.replace("host = \"192.168.50.10\"", "host = \"https://nas\"");
    assert!(matches!(
        decode_device_toml(&text),
        Err(ConfigIoError::ModelValidation(
            ModelError::HostContainsSchemeOrPath
        ))
    ));
}
