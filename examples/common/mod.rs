//! 课程共享的虚构样例，只由 examples 中的示例程序使用。

use nasnet::model::{
    Address, ConfigError, ConnectionConfig, Device, NfsAuthSys, NfsConfig, NfsVersion,
    ProtocolConnection, SmbAuth, SmbConfig, SmbDialect,
};

pub fn sample_device() -> Result<Device, ConfigError> {
    let name = String::from("我的 NAS");
    let mut device = Device::new("nas-1".into(), name);
    for (id, host, priority) in [
        ("address-1", "192.168.50.10", 2),
        ("address-2", "nas.example.com", 1),
    ] {
        device.try_add_address(Address {
            id: id.into(),
            host: host.into(),
            priority,
        })?;
    }
    device.try_add_connection(ProtocolConnection {
        id: "nfs-data".into(),
        config: ConnectionConfig::Nfs(NfsConfig {
            export_path: "/volume1/data".into(),
            version: NfsVersion::V3,
            port: None,
            authentication: NfsAuthSys {
                uid: 1000,
                gid: 1000,
                supplementary_groups: vec![100],
            },
        }),
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

pub fn show_device(device: &Device) {
    println!("设备：{}（{}）", device.name(), device.id());
    for address in device.addresses() {
        println!(
            "  {}：{}，优先级 {}",
            address.id, address.host, address.priority
        );
    }
    for connection in device.connections() {
        match &connection.config {
            ConnectionConfig::Nfs(config) => println!(
                "连接 {}：NFS {:?}，导出 {}",
                connection.id, config.version, config.export_path
            ),
            ConnectionConfig::Smb(config) => println!(
                "连接 {}：SMB {:?}，共享 {}，密码 [隐藏]",
                connection.id, config.dialect, config.share_name
            ),
        }
    }
}
