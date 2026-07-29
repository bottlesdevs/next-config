use next_config::{Config, Migration, error::Error, load, submit_migration};
use serde::{Deserialize, Serialize};
use serde_value::Value;

#[derive(Debug, PartialEq, Serialize, Deserialize, Config)]
#[config(version = 2)]
struct Migrated {
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

submit_migration!(Migrated, V1ToV2);

#[tokio::test]
async fn migrates_and_persists() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    std::fs::write(&path, "_version = 1\nname = 'old'\n").unwrap();

    assert_eq!(
        load::<Migrated>(&path).await.unwrap(),
        Migrated {
            name: "old".into(),
            enabled: true
        }
    );
    assert!(
        std::fs::read_to_string(path)
            .unwrap()
            .contains("_version = 2")
    );
}

#[derive(Serialize, Deserialize, Config)]
#[config(version = 2)]
struct Missing {
    value: u32,
}

#[tokio::test]
async fn rejects_missing_migration() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    std::fs::write(&path, "_version = 1\nvalue = 1\n").unwrap();

    assert!(matches!(
        load::<Missing>(path).await,
        Err(Error::MissingMigration(1))
    ));
}
