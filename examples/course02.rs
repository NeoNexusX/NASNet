mod common;

use nasnet::model::{Address, ConfigError};

fn main() -> Result<(), ConfigError> {
    let candidate = Address {
        id: "candidate".into(),
        host: String::new(),
        priority: 1,
    };
    match candidate.validate() {
        Ok(()) => println!("候选地址可以交给设备保存"),
        Err(error) => println!("Candidate rejected: {error}"),
    }
    let device = common::sample_device()?;
    common::show_device(&device);
    Ok(())
}
