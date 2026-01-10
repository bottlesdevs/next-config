# next-config

A flexible, type-safe configuration system for Rust with versioning and migrations.

## Quick Start

### 1. Define Your Config

```rust
use next_config::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, Config)]
#[serde(default)]
#[config(version = 1, file_name = "app.toml")]
struct AppConfig {
    name: String,
    port: u16,
    debug: bool,
}
```

### 2. Use the Config Store

```rust
use next_config::ConfigStore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the store
    let mut store = ConfigStore::init("./config")?;
    
    // Load all registered configs
    store.load_all()?;
    
    // Read config
    let config = store.get::<AppConfig>()?;
    println!("App: {} running on port {}", config.name, config.port);
    
    // Update config
    store.update::<AppConfig, _>(|cfg| {
        cfg.port = 9090;
        cfg.debug = true;
        Ok(())
    })?;
    
    Ok(())
}
```

## Advanced Features

### Versioning and Migrations

When you need to change your config schema, increment the version and define a migration:

```rust
use next_config::{Config, Migration, submit_migration, error::Error};
use serde::{Deserialize, Serialize};
use serde_value::Value;

// Updated config (version 2)
#[derive(Debug, Default, Serialize, Deserialize, Config)]
#[serde(default)]
#[config(version = 2, file_name = "app.toml")]
struct AppConfig {
    name: String,
    port: u16,
    debug: bool,
    max_connections: u32,  // New field in v2
}

// Migration from version 1 to 2
struct AppConfigV1ToV2;

impl Migration for AppConfigV1ToV2 {
    const FROM: u32 = 1;

    fn migrate(value: &mut Value) -> Result<(), Error> {
        // Add new field with default value
        if let Value::Map(map) = value {
            map.insert(
                Value::String("max_connections".to_string()),
                Value::U32(100),
            );
        }
        Ok(())
    }
}

// Register the migration
submit_migration!(AppConfig, AppConfigV1ToV2);
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
