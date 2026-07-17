use next_config::{Config, load, save};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Config)]
#[config(version = 1)]
struct AppConfig {
    name: String,
    port: u16,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join("next-config-example.toml");
    save(
        &path,
        &AppConfig {
            name: "example".into(),
            port: 8080,
        },
    )?;
    println!("{:?}", load::<AppConfig>(path)?);
    Ok(())
}
