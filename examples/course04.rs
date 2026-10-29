use nasnet::store::StoreConnection;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/configuration/config.toml");
    let store = StoreConnection::from_config_path(path);
    let config = store.load_app_required()?;
    println!("Language: {}", config.software.language);
    println!(
        "Console: {}, file: {}",
        config.logging.console_level, config.logging.file_level
    );
    for reference in config.devices {
        println!("{} -> {}", reference.id, reference.path.display());
    }
    Ok(())
}
