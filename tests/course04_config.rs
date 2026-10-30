mod common;

use common::{APP_CONFIG, TestResult};
use nasnet::config::{
    AppConfig, ConfigError, ConfigIoError, DeviceReference, SCHEMA_VERSION, decode_app_config,
};
use std::error::Error;

#[test]
fn default_config_is_valid() -> TestResult {
    let config = AppConfig::default();
    config.validate()?;
    assert_eq!(config.schema_version, SCHEMA_VERSION);
    assert_eq!(config.logging.console_level, "info");
    assert_eq!(config.logging.file_level, "trace");
    assert_eq!(config.logging.file_path.to_str(), Some("logs/nasnet.log"));
    assert!(config.devices.is_empty());
    Ok(())
}

#[test]
fn decodes_example_settings_and_preserves_reference_order() -> TestResult {
    let config = decode_app_config(APP_CONFIG)?;
    assert_eq!(config.software.language, "zh-CN");
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
        "language = \"zh-CN\"",
        "language = \"zh-CN\"\nunknown = true",
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
    use nasnet::{
        config::DeviceData,
        model::{Address, Device, ModelError},
        store::StoreError,
    };

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
