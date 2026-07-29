use next_config::{Config, error::Error, load, save};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Config)]
#[config(version = 1)]
struct TestConfig {
    name: String,
    count: u32,
}

#[test]
fn saves_and_loads_any_path() {
    futures_lite::future::block_on(async {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("nested/config.toml");
        async_fs::create_dir(path.parent().unwrap()).await.unwrap();
        let expected = TestConfig {
            name: "test".into(),
            count: 42,
        };

        save(&path, &expected).await.unwrap();

        assert_eq!(load::<TestConfig>(&path).await.unwrap(), expected);
        assert!(
            std::fs::read_to_string(path)
                .unwrap()
                .contains("_version = 1")
        );
        assert!(!root.path().join("nested/config.tmp").exists());
    });
}

#[derive(Serialize, Deserialize, Config)]
#[config(version = 1)]
struct OlderConfig {
    value: u32,
}

#[test]
fn rejects_newer_versions() {
    futures_lite::future::block_on(async {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("config.toml");
        std::fs::write(&path, "_version = 2\nvalue = 1\n").unwrap();

        assert!(matches!(
            load::<OlderConfig>(path).await,
            Err(Error::UnsupportedVersion {
                found: 2,
                supported: 1
            })
        ));
    });
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Config)]
#[serde(deny_unknown_fields)]
#[config(version = 1)]
struct Strict {
    value: u32,
}

#[test]
fn version_metadata_is_not_deserialized_as_a_config_field() {
    futures_lite::future::block_on(async {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("strict.toml");
        save(&path, &Strict { value: 7 }).await.unwrap();
        assert_eq!(load::<Strict>(path).await.unwrap(), Strict { value: 7 });
    });
}
