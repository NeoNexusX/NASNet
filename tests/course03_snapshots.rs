#[path = "common/model.rs"]
mod fixtures;

use nasnet::{
    config::DeviceData,
    model::{Address, ConnectionConfig, Device, ModelError},
};

#[test]
fn valid_address_update_preserves_its_position() {
    let mut device = fixtures::device();
    device
        .try_update_address(Address {
            id: "address-1".into(),
            host: "192.168.50.20".into(),
            priority: 0,
        })
        .unwrap();
    assert_eq!(device.addresses()[0].id, "address-1");
    assert_eq!(device.addresses()[0].host, "192.168.50.20");
    assert_eq!(device.addresses()[0].priority, 0);
    assert_eq!(device.addresses()[1].id, "address-2");
}

#[test]
fn invalid_address_update_keeps_original_values() {
    let mut device = fixtures::device();
    assert!(matches!(
        device.try_update_address(Address {
            id: "address-1".into(),
            host: String::new(),
            priority: 99
        }),
        Err(ModelError::EmptyHost)
    ));
    assert_eq!(device.addresses()[0].host, "192.168.50.10");
    assert_eq!(device.addresses()[0].priority, 1);
}

#[test]
fn missing_address_update_distinguishes_invalid_input() {
    let mut device = fixtures::device();
    assert!(
        matches!(device.try_update_address(Address { id: "missing".into(), host: "nas.example.com".into(), priority: 1 }), Err(ModelError::AddressNotFound(id)) if id == "missing")
    );
    assert!(matches!(
        device.try_update_address(Address {
            id: "missing".into(),
            host: String::new(),
            priority: 1
        }),
        Err(ModelError::EmptyHost)
    ));
    assert_eq!(device.address_count(), 2);
}

#[test]
fn removing_an_address_returns_ownership_and_preserves_other_order() {
    let mut device = fixtures::device();
    let removed = device.remove_address("address-1").unwrap();
    assert_eq!(removed.host, "192.168.50.10");
    assert_eq!(device.addresses()[0].id, "address-2");
    assert!(matches!(
        device.remove_address("address-1"),
        Err(ModelError::AddressNotFound(_))
    ));
    let mut empty = Device::new("empty".into(), "Empty".into());
    assert!(matches!(
        empty.remove_address("missing"),
        Err(ModelError::AddressNotFound(_))
    ));
}

#[test]
fn connection_borrow_outlives_the_query_string() {
    let device = fixtures::device();
    let borrowed = {
        let query = String::from("nfs-data");
        device.find_connection(&query)
    };
    assert_eq!(borrowed.unwrap().id, "nfs-data");
    assert!(device.find_connection("missing").is_none());
}

#[test]
fn valid_connection_update_preserves_position_and_other_connections() {
    let mut device = fixtures::device();
    let mut replacement = device.connections()[0].clone();
    if let ConnectionConfig::Nfs(config) = &mut replacement.config {
        config.export_path = "/archive".into();
    }
    device.try_update_connection(replacement).unwrap();
    assert!(
        matches!(&device.connections()[0].config, ConnectionConfig::Nfs(config) if config.export_path == "/archive")
    );
    assert_eq!(device.connections()[1].id, "smb-data");
}

#[test]
fn invalid_connection_update_keeps_original_values() {
    let mut device = fixtures::device();
    let mut replacement = device.connections()[1].clone();
    if let ConnectionConfig::Smb(config) = &mut replacement.config {
        config.authentication.username.clear();
    }
    assert!(matches!(
        device.try_update_connection(replacement),
        Err(ModelError::EmptySmbUsername)
    ));
    assert!(
        matches!(&device.connections()[1].config, ConnectionConfig::Smb(config) if config.authentication.username == "nas-user")
    );
}

#[test]
fn missing_connection_update_and_removal_keep_saved_values() {
    let mut device = fixtures::device();
    let mut replacement = device.connections()[0].clone();
    replacement.id = "missing".into();
    assert!(matches!(
        device.try_update_connection(replacement),
        Err(ModelError::ConnectionNotFound(_))
    ));
    let removed = device.remove_connection("nfs-data").unwrap();
    assert_eq!(removed.id, "nfs-data");
    assert_eq!(device.connections()[0].id, "smb-data");
    assert!(matches!(
        device.remove_connection("nfs-data"),
        Err(ModelError::ConnectionNotFound(_))
    ));
    let mut empty = Device::new("empty".into(), "Empty".into());
    assert!(matches!(
        empty.remove_connection("missing"),
        Err(ModelError::ConnectionNotFound(_))
    ));
}

#[test]
fn snapshot_restoration_preserves_order_protocols_and_password() {
    let original = fixtures::device();
    let restored = Device::try_from(DeviceData::from(&original)).unwrap();
    assert_eq!(restored.id(), original.id());
    assert_eq!(restored.name(), original.name());
    assert_eq!(
        restored
            .addresses()
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        ["address-1", "address-2"]
    );
    assert_eq!(
        restored
            .connections()
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        ["nfs-data", "smb-data"]
    );
    assert!(
        matches!(&restored.connections()[0].config, ConnectionConfig::Nfs(config) if config.authentication.supplementary_groups == [100] && config.port.is_none())
    );
    assert!(
        matches!(&restored.connections()[1].config, ConnectionConfig::Smb(config) if config.authentication.password == " demo-only-password " && config.authentication.domain.is_none())
    );
}

#[test]
fn snapshot_edits_do_not_mutate_the_original_device() {
    let original = fixtures::device();
    let mut snapshot = DeviceData::from(&original);
    snapshot.name = "Changed snapshot".into();
    snapshot.addresses[0].host = "other.example.com".into();
    let restored = Device::try_from(snapshot).unwrap();
    assert_eq!(restored.name(), "Changed snapshot");
    assert_eq!(original.name(), "Test NAS");
    assert_eq!(original.addresses()[0].host, "192.168.50.10");
}

#[test]
fn restoration_rejects_blank_host() {
    let mut snapshot = DeviceData::from(&fixtures::device());
    snapshot.addresses[0].host.clear();
    assert!(matches!(
        Device::try_from(snapshot),
        Err(ModelError::EmptyHost)
    ));
}

#[test]
fn restoration_rejects_duplicate_address_ids() {
    let mut snapshot = DeviceData::from(&fixtures::device());
    snapshot.addresses.push(snapshot.addresses[0].clone());
    assert!(matches!(
        Device::try_from(snapshot),
        Err(ModelError::DuplicateAddressId(_))
    ));
}

#[test]
fn restoration_rejects_duplicate_connection_ids() {
    let mut snapshot = DeviceData::from(&fixtures::device());
    snapshot.connections.push(snapshot.connections[0].clone());
    assert!(matches!(
        Device::try_from(snapshot),
        Err(ModelError::DuplicateConnectionId(_))
    ));
}

#[test]
fn restoration_revalidates_protocol_settings() {
    let mut snapshot = DeviceData::from(&fixtures::device());
    if let ConnectionConfig::Nfs(config) = &mut snapshot.connections[0].config {
        config.export_path = "relative/path".into();
    }
    assert!(matches!(
        Device::try_from(snapshot),
        Err(ModelError::InvalidNfsExportPath)
    ));
    for case in 0..3 {
        let mut snapshot = DeviceData::from(&fixtures::device());
        if let ConnectionConfig::Smb(config) = &mut snapshot.connections[1].config {
            match case {
                0 => config.authentication.username.clear(),
                1 => config.authentication.password.clear(),
                _ => config.port = Some(0),
            }
        }
        let error = Device::try_from(snapshot)
            .err()
            .expect("invalid snapshot must fail");
        assert!(matches!(
            (case, error),
            (0, ModelError::EmptySmbUsername)
                | (1, ModelError::EmptySmbPassword)
                | (2, ModelError::InvalidPort)
        ));
    }
}

#[test]
fn restoration_rejects_blank_address_and_connection_ids() {
    let mut snapshot = DeviceData::from(&fixtures::device());
    snapshot.addresses[0].id.clear();
    assert!(matches!(
        Device::try_from(snapshot),
        Err(ModelError::EmptyAddressId)
    ));
    let mut snapshot = DeviceData::from(&fixtures::device());
    snapshot.connections[0].id.clear();
    assert!(matches!(
        Device::try_from(snapshot),
        Err(ModelError::EmptyConnectionId)
    ));
}

#[test]
fn empty_device_snapshot_restores_empty_lists() {
    let empty = Device::new("empty".into(), "Empty".into());
    let restored = Device::try_from(DeviceData::from(&empty)).unwrap();
    assert_eq!(restored.address_count(), 0);
    assert!(restored.connections().is_empty());
}
