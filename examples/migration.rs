use next_config::{Config, ConfigStore, Migration, error::Error, submit_config, submit_migration};
use serde::{Deserialize, Serialize};
use serde_value::Value;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct ServerConfig {
    host: String,
    port: u16,
    max_connections: u32,
    use_tls: bool, // added in v2
}

impl Config for ServerConfig {
    const VERSION: u32 = 2;
    const FILE_NAME: &'static str = "server.toml";
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
            max_connections: 100,
            use_tls: false,
        }
    }
}

submit_config!(ServerConfig);

struct ServerConfigV1ToV2;

impl Migration for ServerConfigV1ToV2 {
    const FROM: u32 = 1;

    fn migrate(value: &mut Value) -> Result<(), Error> {
        if let Value::Map(map) = value {
            map.insert(Value::String("use_tls".to_string()), Value::Bool(true));
        }
        Ok(())
    }
}

submit_migration!(ServerConfig, ServerConfigV1ToV2);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let config_dir = temp_dir.path();

    // Write a v1 config file (without use_tls field)
    let v1_config = r#"
_version = 1
host = "production.example.com"
port = 443
max_connections = 1000
"#;

    println!("Before migration:");
    println!("{}", v1_config);

    let config_path = config_dir.join("server.toml");
    std::fs::write(&config_path, v1_config)?;

    // Create a new store and load - this will trigger migration
    let mut store = ConfigStore::init(config_dir)?;
    store.load::<ServerConfig>()?;

    let migrated_config = store.get::<ServerConfig>()?;
    println!("After migration:");
    println!("{:#?}", migrated_config);

    Ok(())
}
