use nasnet::config::{ConfigDocument, DeviceData, SCHEMA_VERSION, decode_device, encode_device};
use nasnet::model::{
    Address, ConfigError, ConnectionConfig, Device, NfsAuthSys, NfsConfig, NfsVersion,
    ProtocolConnection, SmbAuth, SmbConfig, SmbDialect,
};
use std::convert::TryFrom;

fn show_device(device: &Device) {
    println!("设备：{}（{}）", device.name(), device.id());
    println!("地址数量：{}", device.address_count());

    for address in device.addresses() {
        println!(
            "  {}：{}，优先级 {}",
            address.id, address.host, address.priority
        );
    }
}

pub fn run() {
    println!("第 01 课 A：设备与地址（内存演示）");

    let name = String::from("我的 NAS");
    let mut device = Device::new(String::from("nas-1"), name);

    let first_address = Address {
        id: String::from("address-1"),
        host: String::from("192.168.50.10"),
        priority: 2,
    };

    device
        .try_add_address(first_address)
        .expect("Sample address must pass validation");
    device
        .try_add_address(Address {
            id: String::from("address-2"),
            host: String::from("nas.example.com"),
            priority: 1,
        })
        .expect("Sample address must pass validation");

    println!("\n添加后，按添加顺序展示：");
    show_device(&device);

    let name_ref = device.name();
    println!("\n修改前借用名称：{name_ref}");
    device.rename(String::from("家里的 NAS"));

    println!("\n修改后，地址仍归这台设备所有：");
    show_device(&device);
}

pub fn run_lesson02() {
    println!("第 01 课 B：地址查找与优先级排序（内存演示）");
    let mut device = Device::new(String::from("nas-1"), String::from("我的 NAS"));
    for (id, host, priority) in [
        ("address-1", "192.168.50.10", 2),
        ("address-2", "nas.example.com", 1),
        ("address-3", "10.20.30.10", 1),
    ] {
        device
            .try_add_address(Address {
                id: String::from(id),
                host: String::from(host),
                priority,
            })
            .expect("Sample address must pass validation");
    }

    println!("\n按 ID 查找：");
    for id in ["address-2", "missing"] {
        match device.find_address(id) {
            Some(address) => println!("{id} → {}", address.host),
            None => println!("{id} → 未找到"),
        }
    }

    println!("\n按优先级排列的候选地址：");
    for address in device.addresses_by_priority() {
        println!(
            "  {}：{}，优先级 {}",
            address.id, address.host, address.priority
        );
    }

    println!("\n保存顺序保持原样：");
    show_device(&device);
}

pub fn run_lesson03() {
    println!("第 02 课 A：配置校验与 Result（内存演示）");

    println!("\nHost 基础校验：");
    for host in [
        "192.168.50.10",
        "nas.example.com",
        "2001:db8::10",
        "",
        "   ",
        "nas example.com",
        " nas.example.com",
        "nas.example.com ",
        "nas\texample.com",
        "nas.example.com\n",
        "smb://nas.example.com",
        "nas.example.com/data",
        "nas.example.com\\data",
    ] {
        let address = Address {
            id: String::from("host-check"),
            host: host.to_owned(),
            priority: 0,
        };
        match address.validate() {
            Ok(()) => println!("{host:?} → 基础校验通过"),
            Err(error) => println!("{host:?} → {error}"),
        }
    }

    println!("\n单条地址校验：");
    for (id, host) in [
        ("address-1", "192.168.50.10"),
        ("", "nas.example.com"),
        ("   ", "nas.example.com"),
        ("address-empty-host", ""),
        ("address-with-path", "nas.example.com/data"),
    ] {
        let address = Address {
            id: String::from(id),
            host: String::from(host),
            priority: 0,
        };
        match address.validate() {
            Ok(()) => println!("地址 {id:?} → 校验通过"),
            Err(error) => println!("地址 {id:?} → {error}"),
        }
    }

    println!("\n通过校验入口添加地址：");
    let mut device = Device::new(String::from("nas-1"), String::from("我的 NAS"));
    for (id, host, priority) in [
        ("address-1", "192.168.50.10", 2),
        ("address-1", "nas.example.com", 1),
        ("address-2", "", 1),
        ("address-3", "nas.example.com", 1),
    ] {
        println!("尝试添加 {id:?}，Host 为 {host:?}：");
        let address = Address {
            id: String::from(id),
            host: String::from(host),
            priority,
        };
        match device.try_add_address(address) {
            Ok(()) => println!("  添加成功"),
            Err(error) => println!("  Add failed: {error}"),
        }
        println!("  当前地址数量：{}", device.address_count());
    }

    println!("\n最后保存的地址：");
    show_device(&device);
}

fn nfs_example(export_path: &str, version: NfsVersion, port: Option<u16>) -> NfsConfig {
    NfsConfig {
        export_path: String::from(export_path),
        version,
        port,
        authentication: NfsAuthSys {
            uid: 1000,
            gid: 1000,
            supplementary_groups: vec![100],
        },
    }
}

fn smb_example(share_name: &str, dialect: SmbDialect) -> SmbConfig {
    SmbConfig {
        share_name: String::from(share_name),
        base_path: Some(String::from("photos/2026")),
        port: Some(445),
        authentication: SmbAuth {
            username: String::from("nas-user"),
            domain: None,
            password: String::from("demo-only-password"),
        },
        dialect,
    }
}

fn show_connections(device: &Device) {
    for connection in device.connections() {
        println!("连接 {}：", connection.id);
        match &connection.config {
            ConnectionConfig::Nfs(config) => {
                println!(
                    "  NFS {:?}，导出 {}，端口 {:?}",
                    config.version, config.export_path, config.port
                );
                println!(
                    "  AUTH_SYS：uid={}，gid={}，附加组={:?}",
                    config.authentication.uid,
                    config.authentication.gid,
                    config.authentication.supplementary_groups
                );
            }
            ConnectionConfig::Smb(config) => {
                println!(
                    "  SMB {:?}，共享 {}，起始目录 {:?}，端口 {:?}",
                    config.dialect, config.share_name, config.base_path, config.port
                );
                println!(
                    "  账号 {}，域 {:?}，密码 [隐藏]",
                    config.authentication.username, config.authentication.domain
                );
            }
        }
    }
}

pub fn run_lesson04() {
    println!("第 02 课 B：独立的 NFS/SMB 配置（内存演示）");

    println!("\nNFS 配置校验：");
    for config in [
        nfs_example("/volume1/data", NfsVersion::V3, None),
        nfs_example("/", NfsVersion::V4, Some(2049)),
        nfs_example("", NfsVersion::V3, None),
        nfs_example("volume1/data", NfsVersion::V3, None),
        nfs_example("/volume1/data", NfsVersion::V3, Some(0)),
        nfs_example("/volume1/\0data", NfsVersion::V3, None),
    ] {
        match config.validate() {
            Ok(()) => println!(
                "{:?}，端口 {:?} → 格式校验通过",
                config.export_path, config.port
            ),
            Err(error) => println!("{:?}，端口 {:?} → {error}", config.export_path, config.port),
        }
    }

    println!("\nSMB 配置校验：");
    let mut empty_username = smb_example("data", SmbDialect::Auto);
    empty_username.authentication.username = String::new();
    let mut empty_password = smb_example("data", SmbDialect::Auto);
    empty_password.authentication.password = String::new();
    let mut zero_port = smb_example("data", SmbDialect::Auto);
    zero_port.port = Some(0);
    let mut absolute_base_path = smb_example("data", SmbDialect::Auto);
    absolute_base_path.base_path = Some(String::from("/photos"));
    let mut root = smb_example("data", SmbDialect::Auto);
    root.base_path = None;
    root.port = None;

    for config in [
        smb_example("data", SmbDialect::Auto),
        root,
        smb_example("team data", SmbDialect::Smb2),
        smb_example("", SmbDialect::Auto),
        smb_example("data/photos", SmbDialect::Smb3),
        absolute_base_path,
        empty_username,
        empty_password,
        zero_port,
    ] {
        match config.validate() {
            Ok(()) => println!("共享 {:?} → 格式校验通过", config.share_name),
            Err(error) => println!("共享 {:?} → {error}", config.share_name),
        }
    }

    println!("\n为同一台设备保存独立的协议连接：");
    let mut device = Device::new(String::from("nas-1"), String::from("我的 NAS"));
    if let Err(error) = device.try_add_address(Address {
        id: String::from("address-1"),
        host: String::from("192.168.50.10"),
        priority: 1,
    }) {
        eprintln!("Failed to prepare sample addresses: {error}");
        return;
    }
    for connection in [
        ProtocolConnection {
            id: String::from("nfs-data"),
            config: ConnectionConfig::Nfs(nfs_example("/volume1/data", NfsVersion::V3, None)),
        },
        ProtocolConnection {
            id: String::from("smb-data"),
            config: ConnectionConfig::Smb(smb_example("data", SmbDialect::Auto)),
        },
        ProtocolConnection {
            id: String::from("nfs-data"),
            config: ConnectionConfig::Nfs(nfs_example("/volume1/backup", NfsVersion::V3, None)),
        },
        ProtocolConnection {
            id: String::new(),
            config: ConnectionConfig::Smb(smb_example("data", SmbDialect::Auto)),
        },
        ProtocolConnection {
            id: String::from("invalid-nfs"),
            config: ConnectionConfig::Nfs(nfs_example("relative/path", NfsVersion::V3, None)),
        },
        ProtocolConnection {
            id: String::from("invalid-smb"),
            config: ConnectionConfig::Smb(smb_example("data/photos", SmbDialect::Auto)),
        },
    ] {
        println!("尝试添加连接 {:?}：", connection.id);
        match device.try_add_connection(connection) {
            Ok(()) => println!("  添加成功"),
            Err(error) => println!("  Add failed: {error}"),
        }
        println!("  当前连接数量：{}", device.connections().len());
    }

    println!("\n保存的设备与连接：");
    show_device(&device);
    show_connections(&device);
}

fn show_result(label: &str, result: Result<(), ConfigError>) {
    match result {
        Ok(()) => println!("{label} → 成功"),
        Err(error) => println!("{label} → Failed: {error}"),
    }
}

pub fn run_lesson05() {
    println!("NASNet 第 3 课 A：模型封装与生命周期（演示编号 lesson05）");
    let mut device = Device::new(String::from("nas-1"), String::from("我的 NAS"));
    for (id, host, priority) in [
        ("address-1", "192.168.50.10", 2),
        ("address-2", "nas.example.com", 1),
        ("address-3", "10.20.30.10", 1),
    ] {
        device
            .try_add_address(Address {
                id: id.into(),
                host: host.into(),
                priority,
            })
            .expect("Initial sample addresses must be valid and have unique IDs");
    }
    for (id, config) in [
        (
            "nfs-data",
            ConnectionConfig::Nfs(nfs_example("/volume1/data", NfsVersion::V3, None)),
        ),
        (
            "smb-data",
            ConnectionConfig::Smb(smb_example("data", SmbDialect::Auto)),
        ),
        (
            "smb-backup",
            ConnectionConfig::Smb(smb_example("backup", SmbDialect::Smb3)),
        ),
    ] {
        device
            .try_add_connection(ProtocolConnection {
                id: id.into(),
                config,
            })
            .expect("Initial sample connections must be valid and have unique IDs");
    }

    println!("\n地址更新：每次操作后都显示保存结果");
    for (id, host, priority) in [
        ("address-1", "192.168.50.20", 1),
        ("address-1", "", 99),
        ("missing", "nas.example.com", 0),
        ("missing", "", 0),
    ] {
        println!("尝试更新 {id:?}，Host={host:?}，优先级={priority}");
        show_result(
            "更新地址",
            device.try_update_address(Address {
                id: id.into(),
                host: host.into(),
                priority,
            }),
        );
        show_device(&device);
    }
    println!("\n同优先级按保存顺序排列：");
    for address in device.addresses_by_priority() {
        println!("  {}", address.id);
    }

    println!("\n地址删除：");
    for id in ["address-1", "address-1"] {
        match device.remove_address(id) {
            Ok(removed) => println!("删除成功，取回地址 {}：{}", removed.id, removed.host),
            Err(error) => println!("Failed to delete {id:?}: {error}"),
        }
        show_device(&device);
    }

    println!("\n借用查找连接：");
    let borrowed_connection = {
        let query_id = String::from("nfs-data");
        device.find_connection(&query_id)
    };
    if let Some(found) = borrowed_connection {
        println!("查询字符串已释放，仍可读取设备中的连接：{}", found.id);
    }
    for id in ["smb-data", "missing"] {
        match device.find_connection(id) {
            Some(found) => println!("{id:?} → 找到 {}", found.id),
            None => println!("{id:?} → 未找到"),
        }
    }

    println!("\n连接更新：每次操作后都显示保存结果");
    let mut invalid_smb = smb_example("data", SmbDialect::Auto);
    invalid_smb.authentication.username = String::new();
    for (id, config) in [
        (
            "nfs-data",
            ConnectionConfig::Nfs(nfs_example("/volume1/archive", NfsVersion::V4, Some(2049))),
        ),
        (
            "nfs-data",
            ConnectionConfig::Nfs(nfs_example("relative/path", NfsVersion::V3, None)),
        ),
        (
            "smb-data",
            ConnectionConfig::Smb(smb_example("team data", SmbDialect::Smb3)),
        ),
        ("smb-data", ConnectionConfig::Smb(invalid_smb)),
        (
            "missing",
            ConnectionConfig::Nfs(nfs_example("/data", NfsVersion::V3, None)),
        ),
        (
            "missing",
            ConnectionConfig::Nfs(nfs_example("relative/path", NfsVersion::V3, None)),
        ),
    ] {
        println!("尝试更新连接 {id:?}：");
        show_result(
            "更新连接",
            device.try_update_connection(ProtocolConnection {
                id: id.into(),
                config,
            }),
        );
        show_connections(&device);
    }

    println!("\n连接删除：");
    for id in ["nfs-data", "nfs-data"] {
        match device.remove_connection(id) {
            Ok(removed) => println!("删除成功，取回连接 {}", removed.id),
            Err(error) => println!("Failed to delete {id:?}: {error}"),
        }
        show_connections(&device);
    }

    println!("\n空设备删除：");
    let mut empty = Device::new(String::from("empty"), String::from("空设备"));
    match empty.remove_address("missing") {
        Ok(removed) => println!("Unexpectedly retrieved address {}", removed.id),
        Err(error) => println!("Failed to delete address: {error}"),
    }
    match empty.remove_connection("missing") {
        Ok(removed) => println!("Unexpectedly retrieved connection {}", removed.id),
        Err(error) => println!("Failed to delete connection: {error}"),
    }
    println!(
        "空设备剩余：{} 个地址，{} 条连接",
        empty.addresses().len(),
        empty.connections().len()
    );

    println!("\n最后保存的设备与连接：");
    show_device(&device);
    show_connections(&device);
}

pub fn run_lesson06() {
    println!("第 3 课 B：配置快照、JSON 与恢复时校验（内存演示）");
    let mut original = Device::new(String::from("nas-1"), String::from("家里的 NAS"));
    for (id, host) in [
        ("address-1", "192.168.50.10"),
        ("address-2", "nas.example.com"),
    ] {
        original
            .try_add_address(Address {
                id: id.to_owned(),
                host: host.to_owned(),
                priority: 1,
            })
            .expect("Sample address must be valid");
    }
    let mut smb = smb_example("data", SmbDialect::Smb3);
    smb.authentication.password = String::from(" demo-only-password ");
    for connection in [
        ProtocolConnection {
            id: String::from("nfs-data"),
            config: ConnectionConfig::Nfs(nfs_example("/volume1/data", NfsVersion::V3, None)),
        },
        ProtocolConnection {
            id: String::from("smb-data"),
            config: ConnectionConfig::Smb(smb),
        },
    ] {
        original
            .try_add_connection(connection)
            .expect("Sample connection must be valid");
    }

    println!("\n1. 快照与恢复：");
    let snapshot = DeviceData::from(&original);
    let restored = Device::try_from(snapshot).expect("Valid snapshot must restore successfully");
    show_device(&restored);
    show_connections(&restored);

    println!("\n2. JSON 往返：");
    let json = encode_device(&original).expect("Valid device must encode successfully");
    println!("已生成 JSON（含明文密码，不打印内容）");
    let restored = decode_device(&json).expect("Valid JSON must restore successfully");
    show_device(&restored);
    show_connections(&restored);
    let encoded_again =
        encode_device(&restored).expect("Restored configuration must encode successfully");
    println!("往返前后完整 JSON 相同：{}", json == encoded_again);
    if let Some(ProtocolConnection {
        config: ConnectionConfig::Smb(config),
        ..
    }) = restored.find_connection("smb-data")
    {
        println!(
            "密码及首尾空格原样保留：{}",
            config.authentication.password == " demo-only-password "
        );
    }

    println!("\n3. 修改快照后恢复，逐项观察拒绝原因：");
    for case in 0..10 {
        let mut document = ConfigDocument {
            schema_version: SCHEMA_VERSION,
            device: DeviceData::from(&original),
        };
        let label = match case {
            0 => {
                document.device.addresses[0].host.clear();
                "空 Host"
            }
            1 => {
                document
                    .device
                    .addresses
                    .push(document.device.addresses[0].clone());
                "重复地址 ID"
            }
            2 => {
                document
                    .device
                    .connections
                    .push(document.device.connections[0].clone());
                "重复连接 ID"
            }
            3 => {
                if let ConnectionConfig::Nfs(config) = &mut document.device.connections[0].config {
                    config.export_path = String::from("relative/path");
                }
                "非法 NFS 路径"
            }
            4..=6 => {
                if let ConnectionConfig::Smb(config) = &mut document.device.connections[1].config {
                    match case {
                        4 => config.authentication.username.clear(),
                        5 => config.authentication.password.clear(),
                        _ => config.port = Some(0),
                    }
                }
                match case {
                    4 => "空 SMB 用户名",
                    5 => "空 SMB 密码",
                    _ => "零端口",
                }
            }
            7 => {
                document.schema_version = 99;
                "不支持的配置版本"
            }
            8 => {
                document.device.addresses[0].id.clear();
                "空地址 ID"
            }
            _ => {
                document.device.connections[0].id.clear();
                "空连接 ID"
            }
        };
        let invalid =
            serde_json::to_string(&document).expect("Candidate data must serialize successfully");
        show_restore_result(label, &invalid);
    }

    println!("\n4. JSON 格式或结构错误：");
    show_restore_result("截断 JSON", "{");
    show_restore_result("缺少设备字段", r#"{"schema_version":1}"#);
    let mut unknown_protocol: serde_json::Value =
        serde_json::from_str(&json).expect("Valid JSON expected");
    unknown_protocol["device"]["connections"][0]["config"]["protocol"] = "ftp".into();
    show_restore_result("未知协议", &unknown_protocol.to_string());

    println!("\n5. 空设备往返：");
    let empty = Device::new(String::from("empty"), String::from("空设备"));
    let empty_json = encode_device(&empty).expect("Empty device must encode successfully");
    let empty_restored =
        decode_device(&empty_json).expect("Empty device must restore successfully");
    println!(
        "剩余 {} 个地址，{} 条连接",
        empty_restored.address_count(),
        empty_restored.connections().len()
    );

    println!("\n原设备仍保留完整数据：");
    show_device(&original);
    show_connections(&original);
}

fn show_restore_result(label: &str, json: &str) {
    match decode_device(json) {
        Ok(_) => println!("{label} → Unexpected success; check the restore implementation"),
        Err(error) => println!("{label} → {error}"),
    }
}

/// 第 04 课的完整设备样例；身份和密码仅为虚构演示数据。
pub(super) fn persistence_sample() -> Result<Device, ConfigError> {
    let mut device = Device::new("nas-z".to_owned(), "Primary NAS".to_owned());
    for (id, host) in [
        ("address-1", "192.168.50.10"),
        ("address-2", "nas.example.com"),
    ] {
        device.try_add_address(Address {
            id: id.to_owned(),
            host: host.to_owned(),
            priority: 1,
        })?;
    }
    device.try_add_connection(ProtocolConnection {
        id: "nfs-data".to_owned(),
        config: ConnectionConfig::Nfs(nfs_example("/volume1/data", NfsVersion::V3, None)),
    })?;
    let mut smb = smb_example("data", SmbDialect::Smb3);
    smb.authentication.password = " demo-only-password ' ".to_owned();
    device.try_add_connection(ProtocolConnection {
        id: "smb-data".to_owned(),
        config: ConnectionConfig::Smb(smb),
    })?;
    Ok(device)
}
