use next_config::{Config, ConfigStore, error::Error};
use serde::{Deserialize, Serialize};
use std::fs;
use tempfile::TempDir;

/// Helper to create a temporary directory for tests
fn temp_config_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

#[derive(Config, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[config(version = 1, file_name = "basic.toml")]
#[serde(default)]
struct BasicConfig {
    name: String,
    count: u32,
    enabled: bool,
}

impl Default for BasicConfig {
    fn default() -> Self {
        Self {
            name: "default_name".to_string(),
            count: 42,
            enabled: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Config)]
#[config(version = 1, file_name = "strict.toml")]
struct StrictConfig {
    required_field: String,
    also_required: u32,
}

impl Default for StrictConfig {
    fn default() -> Self {
        Self {
            required_field: "default".to_string(),
            also_required: 0,
        }
    }
}

// UnregisteredConfig uses Config directly without derive(Config)
// so it is NOT registered with inventory
#[derive(Debug, Default, Serialize, Deserialize)]
struct UnregisteredConfig {
    field: String,
}

impl next_config::Config for UnregisteredConfig {
    const VERSION: u32 = 1;
    const FILE_NAME: &'static str = "unregistered.toml";
}

// Note: UnregisteredConfig is intentionally NOT registered!

#[derive(Debug, Serialize, Deserialize, Config)]
#[config(version = 1, file_name = "complex.toml")]
struct ComplexConfig {
    nested: NestedStruct,
    items: Vec<String>,
    optional: Option<u32>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct NestedStruct {
    inner_value: i32,
    inner_name: String,
}

impl Default for ComplexConfig {
    fn default() -> Self {
        Self {
            nested: NestedStruct {
                inner_value: 100,
                inner_name: "nested_default".to_string(),
            },
            items: vec!["item1".to_string(), "item2".to_string()],
            optional: None,
        }
    }
}

#[test]
fn test_load_creates_default_config_file() {
    let temp_dir = temp_config_dir();
    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");

    let config_path = temp_dir.path().join("basic.toml");
    assert!(
        !config_path.exists(),
        "Config file should not exist before loading"
    );
    store
        .register::<BasicConfig>()
        .expect("Failed to register config");
    store.load::<BasicConfig>().expect("Failed to load config");
    assert!(
        config_path.exists(),
        "Config file should be created after loading"
    );
}

#[test]
fn test_load_existing_config_file() {
    let temp_dir = temp_config_dir();
    let config_path = temp_dir.path().join("basic.toml");

    let content = r#"
_version = 1
name = "preexisting"
count = 999
enabled = false
"#;
    fs::write(&config_path, content).expect("Failed to write config file");

    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<BasicConfig>()
        .expect("Failed to register config");
    store.load::<BasicConfig>().expect("Failed to load config");

    let config = store.get::<BasicConfig>().expect("Failed to get config");
    assert_eq!(config.name, "preexisting");
    assert_eq!(config.count, 999);
    assert!(!config.enabled);
}

#[test]
fn test_load_all_configs() {
    let temp_dir = temp_config_dir();
    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<BasicConfig>()
        .expect("Failed to register config");

    let result = store.load_all();
    assert!(result.is_ok(), "load_all should succeed");

    let config = store.get::<BasicConfig>();
    assert!(config.is_ok(), "BasicConfig should be loaded");
}

#[test]
fn test_config_without_version_defaults_to_v1() {
    let temp_dir = temp_config_dir();
    let config_path = temp_dir.path().join("basic.toml");

    // Write a config without _version field
    let content = r#"
name = "no_version"
count = 123
enabled = true
"#;
    fs::write(&config_path, content).expect("Failed to write config file");

    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<BasicConfig>()
        .expect("Failed to register config");
    store.load::<BasicConfig>().expect("Failed to load config");

    let config = store.get::<BasicConfig>().expect("Failed to get config");
    assert_eq!(config.name, "no_version");
    assert_eq!(config.count, 123);

    // After loading, the file should have version added
    let updated_content = fs::read_to_string(&config_path).expect("Failed to read config file");
    assert!(
        updated_content.contains("_version"),
        "Version should be added to file"
    );
}

#[test]
fn test_get_config_returns_correct_values() {
    let temp_dir = temp_config_dir();
    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<BasicConfig>()
        .expect("Failed to register config");
    store.load::<BasicConfig>().expect("Failed to load config");

    let config = store.get::<BasicConfig>().expect("Failed to get config");

    assert_eq!(config.name, "default_name");
    assert_eq!(config.count, 42);
    assert!(config.enabled);
}

#[test]
fn test_get_unregistered_config_returns_error() {
    let temp_dir = temp_config_dir();
    let store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");

    let result = store.get::<UnregisteredConfig>();
    assert!(result.is_err(), "Getting unregistered config should fail");

    if let Err(Error::UnregisteredConfig(name)) = result {
        assert_eq!(name, "unregistered.toml");
    } else {
        panic!("Expected UnregisteredConfig error");
    }
}

#[test]
fn test_update_config_persists_changes() {
    let temp_dir = temp_config_dir();
    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<BasicConfig>()
        .expect("Failed to register config");
    store.load::<BasicConfig>().expect("Failed to load config");

    store
        .update::<BasicConfig, _>(|cfg| {
            cfg.name = "updated_name".to_string();
            cfg.count = 100;
            cfg.enabled = false;
            Ok(())
        })
        .expect("Failed to update config");

    // Verify in memory
    let config = store.get::<BasicConfig>().expect("Failed to get config");
    assert_eq!(config.name, "updated_name");
    assert_eq!(config.count, 100);
    assert!(!config.enabled);

    // Verify on disk by reloading
    let mut new_store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    new_store
        .register::<BasicConfig>()
        .expect("Failed to register config");
    new_store
        .load::<BasicConfig>()
        .expect("Failed to load config");
    let reloaded = new_store
        .get::<BasicConfig>()
        .expect("Failed to get config");

    assert_eq!(reloaded.name, "updated_name");
    assert_eq!(reloaded.count, 100);
    assert!(!reloaded.enabled);
}

#[test]
fn test_multiple_sequential_updates() {
    let temp_dir = temp_config_dir();
    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<BasicConfig>()
        .expect("Failed to register config");
    store.load::<BasicConfig>().expect("Failed to load config");

    for i in 0..5 {
        store
            .update::<BasicConfig, _>(|cfg| {
                cfg.count = i;
                Ok(())
            })
            .expect("Failed to update config");

        let config = store.get::<BasicConfig>().expect("Failed to get config");
        assert_eq!(config.count, i);
    }
}

#[test]
fn test_atomic_save_creates_no_temp_files() {
    let temp_dir = temp_config_dir();
    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<BasicConfig>()
        .expect("Failed to register config");
    store.load::<BasicConfig>().expect("Failed to load config");

    store
        .update::<BasicConfig, _>(|cfg| {
            cfg.name = "atomic_test".to_string();
            Ok(())
        })
        .expect("Failed to update config");

    // Check that no .tmp files remain
    let entries: Vec<_> = fs::read_dir(temp_dir.path())
        .expect("Failed to read dir")
        .filter_map(|e| e.ok())
        .collect();

    for entry in &entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        assert!(
            !name_str.ends_with(".tmp"),
            "No .tmp files should remain after save"
        );
    }
}

#[test]
fn test_missing_fields_get_defaults_with_serde_default() {
    let temp_dir = temp_config_dir();
    let config_path = temp_dir.path().join("basic.toml");

    let partial_content = r#"
_version = 1
name = "partial"
"#;
    fs::write(&config_path, partial_content).expect("Failed to write config file");

    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<BasicConfig>()
        .expect("Failed to register config");
    store.load::<BasicConfig>().expect("Failed to load config");

    let config = store.get::<BasicConfig>().expect("Failed to get config");

    // Explicitly set field
    assert_eq!(config.name, "partial");

    assert_eq!(config.count, 42);
    assert!(config.enabled);
}

#[test]
fn test_missing_fields_without_serde_default_fails() {
    let temp_dir = temp_config_dir();
    let config_path = temp_dir.path().join("strict.toml");

    let partial_content = r#"
_version = 1
required_field = "present"
"#;
    fs::write(&config_path, partial_content).expect("Failed to write config file");

    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    let result = store.load::<StrictConfig>();

    assert!(
        result.is_err(),
        "Loading config with missing required field should fail"
    );
}

#[test]
fn test_complex_nested_config() {
    let temp_dir = temp_config_dir();
    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<ComplexConfig>()
        .expect("Failed to register config");
    store
        .load::<ComplexConfig>()
        .expect("Failed to load config");

    let config = store.get::<ComplexConfig>().expect("Failed to get config");

    assert_eq!(config.nested.inner_value, 100);
    assert_eq!(config.nested.inner_name, "nested_default");
    assert_eq!(config.items, vec!["item1", "item2"]);
    assert!(config.optional.is_none());
}

#[test]
fn test_update_nested_config() {
    let temp_dir = temp_config_dir();
    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<ComplexConfig>()
        .expect("Failed to register config");
    store
        .load::<ComplexConfig>()
        .expect("Failed to load config");

    store
        .update::<ComplexConfig, _>(|cfg| {
            cfg.nested.inner_value = 999;
            cfg.items.push("item3".to_string());
            cfg.optional = Some(42);
            Ok(())
        })
        .expect("Failed to update config");

    // Reload and verify
    let mut new_store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    new_store
        .register::<ComplexConfig>()
        .expect("Failed to register config");
    new_store
        .load::<ComplexConfig>()
        .expect("Failed to load config");
    let config = new_store
        .get::<ComplexConfig>()
        .expect("Failed to get config");

    assert_eq!(config.nested.inner_value, 999);
    assert_eq!(config.items, vec!["item1", "item2", "item3"]);
    assert_eq!(config.optional, Some(42));
}
