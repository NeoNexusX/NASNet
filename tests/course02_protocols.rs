#[path = "common/model.rs"]
mod fixtures;

use nasnet::model::{
    Address, ConnectionConfig, ModelError, NfsConfig, NfsVersion, ProtocolConnection, SmbConfig,
};

fn nfs() -> NfsConfig {
    match fixtures::device().connections()[0].config.clone() {
        ConnectionConfig::Nfs(config) => config,
        _ => panic!("fixture must contain NFS"),
    }
}

fn smb() -> SmbConfig {
    match fixtures::device().connections()[1].config.clone() {
        ConnectionConfig::Smb(config) => config,
        _ => panic!("fixture must contain SMB"),
    }
}

fn address(host: &str) -> Address {
    Address {
        id: "candidate".into(),
        host: host.into(),
        priority: 1,
    }
}

#[test]
fn accepts_ip_addresses_and_domain_names() {
    for host in ["192.168.50.10", "nas.example.com", "2001:db8::10"] {
        assert!(address(host).validate().is_ok(), "host: {host}");
    }
}

#[test]
fn rejects_empty_hosts() {
    for host in ["", "   "] {
        assert!(matches!(
            address(host).validate(),
            Err(ModelError::EmptyHost)
        ));
    }
}

#[test]
fn rejects_whitespace_in_hosts() {
    for host in [
        "nas example.com",
        " nas.example.com",
        "nas.example.com ",
        "nas\texample.com",
        "nas.example.com\n",
    ] {
        assert!(
            matches!(
                address(host).validate(),
                Err(ModelError::HostContainsWhitespace)
            ),
            "host: {host:?}"
        );
    }
}

#[test]
fn rejects_schemes_and_paths_in_hosts() {
    for host in [
        "smb://nas.example.com",
        "nas.example.com/data",
        r"nas.example.com\data",
    ] {
        assert!(matches!(
            address(host).validate(),
            Err(ModelError::HostContainsSchemeOrPath)
        ));
    }
}

#[test]
fn validates_address_id_before_host() {
    let mut candidate = address("");
    for id in ["", " "] {
        candidate.id = id.into();
        assert!(matches!(
            candidate.validate(),
            Err(ModelError::EmptyAddressId)
        ));
    }
}

#[test]
fn failed_address_additions_preserve_saved_values() {
    let mut device = fixtures::device();
    assert!(
        matches!(device.try_add_address(Address { id: "address-1".into(), ..address("other.example.com") }), Err(ModelError::DuplicateAddressId(id)) if id == "address-1")
    );
    assert!(matches!(
        device.try_add_address(address("")),
        Err(ModelError::EmptyHost)
    ));
    assert_eq!(device.address_count(), 2);
    assert_eq!(device.addresses()[0].host, "192.168.50.10");
}

#[test]
fn accepts_supported_nfs_versions_and_optional_ports() {
    let mut config = nfs();
    for version in [NfsVersion::V3, NfsVersion::V4] {
        config.version = version;
        config.port = None;
        assert!(config.validate().is_ok());
        config.port = Some(2049);
        assert!(config.validate().is_ok());
    }
}

#[test]
fn rejects_invalid_nfs_export_paths() {
    let mut config = nfs();
    for path in ["", "relative/path", "/volume1/\0data"] {
        config.export_path = path.into();
        assert!(matches!(
            config.validate(),
            Err(ModelError::InvalidNfsExportPath)
        ));
    }
}

#[test]
fn rejects_explicit_zero_ports_for_both_protocols() {
    let mut nfs = nfs();
    nfs.port = Some(0);
    assert!(matches!(nfs.validate(), Err(ModelError::InvalidPort)));
    let mut smb = smb();
    smb.port = Some(0);
    assert!(matches!(smb.validate(), Err(ModelError::InvalidPort)));
}

#[test]
fn smb_accepts_share_root_and_relative_base_paths() {
    let mut config = smb();
    for path in [None, Some(""), Some("photos/2026")] {
        config.base_path = path.map(str::to_owned);
        assert!(config.validate().is_ok());
    }
}

#[test]
fn smb_rejects_invalid_share_names() {
    let mut config = smb();
    for share in ["", " ", "data/photos", r"data\photos", "data\0photos"] {
        config.share_name = share.into();
        assert!(matches!(
            config.validate(),
            Err(ModelError::InvalidSmbShareName)
        ));
    }
}

#[test]
fn smb_rejects_absolute_or_null_containing_base_paths() {
    let mut config = smb();
    for path in ["/photos", r"\photos", "photos\0data"] {
        config.base_path = Some(path.into());
        assert!(matches!(
            config.validate(),
            Err(ModelError::InvalidSmbBasePath)
        ));
    }
}

#[test]
fn smb_rejects_empty_credentials_but_preserves_password_whitespace() {
    let mut config = smb();
    config.authentication.username = " ".into();
    assert!(matches!(
        config.validate(),
        Err(ModelError::EmptySmbUsername)
    ));
    config.authentication.username = "nas-user".into();
    config.authentication.password.clear();
    assert!(matches!(
        config.validate(),
        Err(ModelError::EmptySmbPassword)
    ));
    config.authentication.password = " demo-only-password ".into();
    assert!(config.validate().is_ok());
    assert_eq!(config.authentication.password, " demo-only-password ");
}

#[test]
fn connection_id_is_validated_before_protocol_settings() {
    let mut config = nfs();
    config.export_path.clear();
    let candidate = ProtocolConnection {
        id: String::new(),
        config: ConnectionConfig::Nfs(config),
    };
    assert!(matches!(
        candidate.validate(),
        Err(ModelError::EmptyConnectionId)
    ));
}

#[test]
fn failed_connection_additions_preserve_saved_connections() {
    let mut device = fixtures::device();
    let existing = device.connections()[0].clone();
    assert!(
        matches!(device.try_add_connection(existing), Err(ModelError::DuplicateConnectionId(id)) if id == "nfs-data")
    );
    let mut invalid = nfs();
    invalid.export_path = "relative".into();
    assert!(matches!(
        device.try_add_connection(ProtocolConnection {
            id: "invalid".into(),
            config: ConnectionConfig::Nfs(invalid)
        }),
        Err(ModelError::InvalidNfsExportPath)
    ));
    assert_eq!(device.connections().len(), 2);
    assert_eq!(device.connections()[0].id, "nfs-data");
}
