use next_config::{Config, ConfigStore, error::Error};
use serde::{Deserialize, Serialize};

/// A simple application configuration.
#[derive(Debug, Serialize, Deserialize, Config)]
#[config(version = 1, file_name = "app.toml")]
struct AppConfig {
    app_name: String,
    port: u16,
    debug: bool,
    max_connections: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_name: "MyApp".to_string(),
            port: 8080,
            debug: false,
            max_connections: 100,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let config_dir = temp_dir.path();

    // Initialize the config store
    // This collects all registered config types but doesn't load them yet
    let mut store = ConfigStore::builder()
        .register::<AppConfig>()?
        .init(config_dir);

    // Load all registered configs from disk
    // If a config file doesn't exist, it will be created with default values
    store.load_all()?;

    // Read the config
    let config = store.get::<AppConfig>()?;
    println!("Initial config:");
    println!("{:#?}", config);

    // Update the config
    // The closure receives a mutable reference to the config.
    // Changes are automatically saved to disk after the closure returns.
    store.update::<AppConfig, _>(|cfg| {
        cfg.app_name = "MyAwesomeApp".to_string();
        cfg.port = 100;
        cfg.debug = true;
        cfg.max_connections = 500;
        Ok(())
    })?;

    println!("After update:");
    let config = store.get::<AppConfig>()?;
    println!("{:#?}", config);

    // Error handling with update
    let result = store.update::<AppConfig, _>(|cfg| {
        if cfg.port < 1024 {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Port must be >= 1024 for non-root users",
            )));
        }
        cfg.port = 443; // This won't actually happen due to the error above
        Ok(())
    });

    match result {
        Ok(()) => println!("Update succeeded"),
        Err(e) => println!("Update failed (expected): {}", e),
    }

    Ok(())
}
