//! 应用编排与展示。core library 不负责终端输出或日志订阅。

mod demo;
pub mod logging;

use std::{ffi::OsString, path::Path};

use nasnet::{
    config::{DeviceReference, encode_device_toml},
    error::{AppError, ConfigError},
    model::Device,
    store::{DeviceStore, StoreConnection},
};

pub fn run(arguments: Vec<OsString>) -> Result<(), AppError> {
    let course = arguments
        .first()
        .and_then(|value| value.to_str())
        .unwrap_or("course01");
    if arguments
        .first()
        .is_some_and(|value| value.to_str().is_none())
    {
        return Err(AppError::InvalidArguments(
            "course name must contain valid Unicode",
        ));
    }
    if (course != "course04" && arguments.len() > 1)
        || arguments.len() > 3
        || (arguments.get(1).is_some_and(|value| value == "--intro") && arguments.len() != 2)
    {
        return Err(AppError::InvalidArguments(
            "only course04 accepts --intro or a configuration directory and optional device ID",
        ));
    }
    let span = tracing::info_span!("course", name = course);
    let _entered = span.enter();
    tracing::info!("course started");
    match course {
        "course01" => {
            demo::run();
            demo::run_lesson02();
        }
        "course02" => {
            demo::run_lesson03();
            demo::run_lesson04();
        }
        "course03" => {
            demo::run_lesson05();
            demo::run_lesson06();
        }
        "course04" => {
            run_bridge_course();
            if !arguments.get(1).is_some_and(|value| value == "--intro") {
                let selected = arguments
                    .get(2)
                    .map(|value| {
                        value.to_str().ok_or(AppError::InvalidArguments(
                            "device ID must contain valid Unicode",
                        ))
                    })
                    .transpose()?;
                run_toml_course(arguments.get(1).map(Path::new), selected)?;
            }
        }
        "lesson01" => demo::run(),
        "lesson02" => demo::run_lesson02(),
        "lesson03" => demo::run_lesson03(),
        "lesson04" => demo::run_lesson04(),
        "lesson05" => demo::run_lesson05(),
        "lesson06" => demo::run_lesson06(),
        other => return Err(AppError::UnknownCourse(other.to_owned())),
    }
    tracing::info!("course completed");
    Ok(())
}

fn run_bridge_course() {
    println!("第 04 课 A：项目结构、统一错误与 tracing");
    println!("先阅读 docs/lessons/04-errors-logging-and-toml-storage.md，再逐项解释日志与调用链。");
    tracing::debug!(library = "tracing", "structured debug event");
    tracing::info!(
        devices = 0,
        storage = "config.toml + device TOML files",
        "configuration storage selected"
    );
    // 预期演示错误在应用层捕获；不把模型错误改写成成功的业务结果。
    let error = ConfigError::EmptyHost;
    tracing::warn!(error = %error, expected = true, "validation example rejected");
    println!("完成讲义中的六项阅读与修改练习后，再做理解验收；输出日志不代表已学会。");
}

fn run_toml_course(
    existing_directory: Option<&Path>,
    selected_id: Option<&str>,
) -> Result<(), AppError> {
    if let Some(directory) = existing_directory {
        let store_connection = StoreConnection::new(directory);
        let config = store_connection.load_app()?;
        tracing::info!(
            references = config.devices.len(),
            "application configuration loaded"
        );
        println!("Language: {}", config.software.language);
        for reference in &config.devices {
            println!("{} -> {}", reference.id, reference.path.display());
        }
        if let Some(id) = selected_id {
            let reference = config
                .devices
                .iter()
                .find(|reference| reference.id == id)
                .ok_or_else(|| ConfigError::DeviceReferenceNotFound(id.to_owned()))?;
            let device = store_connection.load_device(reference)?;
            println!("Selected device: {} ({})", device.name(), device.id());
            tracing::info!(device_id = id, "selected device loaded");
        }
        return Ok(());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(std::io::Error::other)?
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("nasnet-toml-{}-{stamp}", std::process::id()));
    std::fs::create_dir(&directory)?;
    let mut store_connection = StoreConnection::new(&directory);
    println!("第 04 课 B：config.toml 与每台设备独立 TOML");
    println!("Config path: {}", store_connection.config_path().display());
    let mut config = store_connection.load_app()?;
    println!("Initial reference count: {}", config.devices.len());
    let mut device = demo::persistence_sample()?;
    let reference = DeviceReference {
        id: device.id().to_owned(),
        path: "devices/home.toml".into(),
    };
    let backup = Device::new("nas-a".to_owned(), "Backup NAS".to_owned());
    let backup_reference = DeviceReference {
        id: backup.id().to_owned(),
        path: "devices/backup.toml".into(),
    };
    let expected = encode_device_toml(&device)?;
    store_connection.register_device(&mut config, reference.clone(), &device)?;
    store_connection.register_device(&mut config, backup_reference.clone(), &backup)?;
    let mut reopened = StoreConnection::new(&directory);
    let restored_config = reopened.load_app()?;
    println!(
        "Reference order: {:?}",
        restored_config
            .devices
            .iter()
            .map(|reference| reference.id.as_str())
            .collect::<Vec<_>>()
    );
    let restored = reopened.load_device(&reference)?;
    println!(
        "Device round trip matches: {}",
        encode_device_toml(&restored)? == expected
    );
    let main_before = std::fs::read(reopened.config_path())?;
    let backup_path = reopened.resolve_device_path(&backup_reference)?;
    let backup_before = std::fs::read(&backup_path)?;
    device.rename("Renamed primary NAS".to_owned());
    reopened.save_device(&reference, &device)?;
    println!(
        "Main file unchanged after device update: {}",
        std::fs::read(reopened.config_path())? == main_before
    );
    println!(
        "Other device file unchanged: {}",
        std::fs::read(&backup_path)? == backup_before
    );
    println!(
        "Reference removed: {}",
        reopened.remove_device_reference(&mut config, device.id())?
    );
    println!(
        "Removed device file retained: {}",
        reopened.resolve_device_path(&reference)?.is_file()
    );
    println!(
        "Repeated removal: {}",
        reopened.remove_device_reference(&mut config, device.id())?
    );
    // 恢复引用保留文件：这是显式重新关联，不覆盖已存在的设备文件。
    config.devices.push(reference);
    reopened.save_app(&config)?;
    println!(
        "Read references: cargo run --locked -- course04 \"{}\"",
        directory.display()
    );
    println!(
        "Read only one device: cargo run --locked -- course04 \"{}\" nas-z",
        directory.display()
    );
    Ok(())
}
