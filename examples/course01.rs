mod common;

use nasnet::model::ConfigError;

fn main() -> Result<(), ConfigError> {
    let mut device = common::sample_device()?;
    let name_ref = device.name();
    println!("修改前借用名称：{name_ref}");
    device.rename("家里的 NAS".into());
    if let Some(address) = device.find_address("address-2") {
        println!("查询结果：{}", address.host);
    }
    for address in device.addresses_by_priority() {
        println!("候选：{}，优先级 {}", address.id, address.priority);
    }
    common::show_device(&device);
    Ok(())
}
