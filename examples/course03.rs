mod common;

use nasnet::{
    config::{ConfigDocument, DeviceData, SCHEMA_VERSION},
    model::{Address, ConfigError, Device},
};

fn main() -> Result<(), ConfigError> {
    let mut device = common::sample_device()?;
    device.try_update_address(Address {
        id: "address-1".into(),
        host: "192.168.50.20".into(),
        priority: 1,
    })?;
    let borrowed = {
        let query_id = String::from("nfs-data");
        device.find_connection(&query_id)
    };
    if let Some(connection) = borrowed {
        println!("查询字符串释放后仍可借用连接：{}", connection.id);
    }
    let removed = device.remove_address("address-2")?;
    println!("删除后取得地址所有权：{}", removed.id);
    let document = ConfigDocument {
        schema_version: SCHEMA_VERSION,
        device: DeviceData::from(&device),
    };
    println!(
        "快照版本：{}；TOML 文档校验见第 04 课",
        document.schema_version
    );
    let restored = Device::try_from(document.device)?;
    common::show_device(&restored);
    Ok(())
}
