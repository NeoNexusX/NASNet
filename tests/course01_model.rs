#[path = "common/model.rs"]
mod fixtures;

use nasnet::model::{Address, Device};

#[test]
fn new_device_owns_its_name_and_starts_empty() {
    let name = String::from("Home NAS");
    let device = Device::new("nas".into(), name);
    assert_eq!(device.id(), "nas");
    assert_eq!(device.name(), "Home NAS");
    assert!(device.addresses().is_empty());
    assert!(device.connections().is_empty());
}

#[test]
fn renaming_preserves_identity_and_saved_addresses() {
    let mut device = fixtures::device();
    device.rename("Renamed NAS".into());
    assert_eq!(device.id(), "nas-1");
    assert_eq!(device.name(), "Renamed NAS");
    assert_eq!(device.address_count(), 2);
    assert_eq!(device.addresses()[0].host, "192.168.50.10");
}

#[test]
fn address_lookup_borrows_saved_values_and_reports_absence() {
    let device = fixtures::device();
    let address = device
        .find_address("address-2")
        .expect("fixture address exists");
    assert_eq!(address.host, "nas.example.com");
    assert!(std::ptr::eq(address, &device.addresses()[1]));
    assert!(device.find_address("missing").is_none());
}

#[test]
fn priority_sort_is_stable_without_changing_saved_order() {
    let mut device = fixtures::device();
    device
        .try_add_address(Address {
            id: "first-choice".into(),
            host: "10.0.0.1".into(),
            priority: 0,
        })
        .unwrap();
    let sorted = device.addresses_by_priority();
    assert_eq!(
        sorted
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        ["first-choice", "address-1", "address-2"]
    );
    assert_eq!(
        device
            .addresses()
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        ["address-1", "address-2", "first-choice"]
    );
}
