//! 前三课共用的合法设备快照来源；所有身份信息均为虚构测试数据。

use nasnet::model::{
    Address, ConnectionConfig, Device, NfsAuthSys, NfsConfig, NfsVersion, ProtocolConnection,
    SmbAuth, SmbConfig, SmbDialect,
};

pub fn device() -> Device {
    let mut device = Device::new("nas-1".into(), "Test NAS".into());
    for (id, host) in [
        ("address-1", "192.168.50.10"),
        ("address-2", "nas.example.com"),
    ] {
        device
            .try_add_address(Address {
                id: id.into(),
                host: host.into(),
                priority: 1,
            })
            .expect("fixture addresses must be valid");
    }
    device
        .try_add_connection(ProtocolConnection {
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
        })
        .expect("fixture NFS configuration must be valid");
    device
        .try_add_connection(ProtocolConnection {
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
        })
        .expect("fixture SMB configuration must be valid");
    device
}
