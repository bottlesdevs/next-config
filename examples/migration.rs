use next_config::{Config, Migration, error::Error, load, submit_migration};
use serde::{Deserialize, Serialize};
use serde_value::Value;

#[derive(Debug, Serialize, Deserialize, Config)]
#[config(version = 2)]
struct AppConfig {
    name: String,
    enabled: bool,
}

struct V1ToV2;

impl Migration for V1ToV2 {
    const FROM: u32 = 1;

    fn migrate(value: &mut Value) -> Result<(), Error> {
        if let Value::Map(map) = value {
            map.insert(Value::String("enabled".into()), Value::Bool(true));
        }
        Ok(())
    }
}

submit_migration!(AppConfig, V1ToV2);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load::<AppConfig>(std::env::args().nth(1).unwrap_or("app.toml".into())).await?;
    println!("{config:?}");
    Ok(())
}
