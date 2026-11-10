//! 第 05 课离线测试夹具与练习。

use nasnet::model::{
    Address, ConnectionConfig, Device, NfsAuthSys, NfsConfig, NfsVersion, ProtocolConnection,
    SmbAuth, SmbConfig, SmbDialect,
};

/// 故意让地址的保存顺序与优先级顺序不同。
fn sample_device() -> Device {
    let mut device = Device::new("nas-home".to_owned(), "Home NAS".to_owned());
    for (id, host, priority) in [
        ("domain", "nas.example.com", 2),
        ("lan", "192.168.50.10", 1),
        ("vpn", "10.8.0.10", 3),
    ] {
        device
            .try_add_address(Address {
                id: id.to_owned(),
                host: host.to_owned(),
                priority,
            })
            .expect("sample address is valid");
    }
    let smb = ConnectionConfig::Smb(SmbConfig {
        share_name: "data".to_owned(),
        base_path: None,
        port: None,
        authentication: SmbAuth {
            username: "nas-user".to_owned(),
            domain: None,
            password: "demo-only-password".to_owned(),
        },
        dialect: SmbDialect::Auto,
    });
    let nfs = ConnectionConfig::Nfs(NfsConfig {
        export_path: "/volume1/media".to_owned(),
        version: NfsVersion::V4,
        port: None,
        authentication: NfsAuthSys {
            uid: 1000,
            gid: 1000,
            supplementary_groups: Vec::new(),
        },
    });
    for (id, config) in [("smb-data", smb), ("nfs-media", nfs)] {
        device
            .try_add_connection(ProtocolConnection {
                id: id.to_owned(),
                config,
            })
            .expect("sample connection is valid");
    }
    device
}

#[test]
fn sample_device_orders_addresses_by_priority() {
    let device = sample_device();
    let ids: Vec<&str> = device
        .addresses_by_priority()
        .iter()
        .map(|address| address.id.as_str())
        .collect();
    assert_eq!(ids, ["lan", "domain", "vpn"]);
    assert!(device.find_connection("smb-data").is_some());
    assert!(device.find_connection("nfs-media").is_some());
}

// TODO P5.1：在这里实现 MemoryBackend 和 MemorySession。

#[test]
#[ignore = "P5.1：补全测试后启用"]
fn p5_1_mock_records_attempts_and_serves_directories() {
    todo!("先验证失败的 host 也会进入尝试记录，再验证会话读目录");
}

#[test]
#[ignore = "P5.2：补全测试后启用"]
fn p5_2_only_network_errors_allow_address_fallback() {
    todo!("比较八种错误的预期回退行为");
}

#[test]
#[ignore = "P5.3：补全测试后启用"]
fn p5_3_fallback_uses_priority_and_stops_on_rejection() {
    todo!("断言尝试顺序以及拒绝时返回的地址和错误");
}

#[test]
#[ignore = "P5.4：补全测试后启用"]
fn p5_4_dispatch_calls_only_the_selected_protocol() {
    todo!("比较两个后端各自的尝试记录");
}

#[test]
#[ignore = "P5.5：补全测试后启用"]
fn p5_5_listing_sorts_entries_and_preserves_errors() {
    todo!("比较目录结果中的名称与顺序，并检查读取失败时的错误");
}
