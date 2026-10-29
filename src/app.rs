//! 项目应用编排：展示主配置，并在明确选择设备时按需加载。

pub mod logging;
pub mod startup;

use nasnet::{
    config::AppConfig,
    error::{AppError, ConfigError},
    store::{DeviceStore, StoreConnection},
};

pub fn run(startup: startup::Startup) -> Result<(), AppError> {
    let span = tracing::info_span!("application");
    let _entered = span.enter();
    tracing::info!("application started");
    show_configuration(
        &startup.store,
        &startup.config,
        startup.selected_id.as_deref(),
    )?;
    tracing::info!("application completed");
    Ok(())
}

fn show_configuration(
    store_connection: &StoreConnection,
    config: &AppConfig,
    selected_id: Option<&str>,
) -> Result<(), AppError> {
    tracing::info!(
        references = config.devices.len(),
        "application configuration loaded"
    );
    println!("Language: {}", config.software.language);
    for reference in &config.devices {
        tracing::trace!(device_id = %reference.id, path = %reference.path.display(), "device reference listed");
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
    Ok(())
}
