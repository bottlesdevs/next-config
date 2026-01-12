//! Integration tests for config migrations.

use next_config::{Config, ConfigStore, Migration, error::Error, submit_migration};
use serde::{Deserialize, Serialize};
use serde_value::Value;
use std::fs;
use tempfile::TempDir;

/// Helper to create a temporary directory for tests
fn temp_config_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config)]
#[config(version = 2, file_name = "migratable.toml")]
struct MigratableConfig {
    name: String,
    value: u32,
    new_field: String, // Added in version 2
}

impl Default for MigratableConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            value: 10,
            new_field: "default_new".to_string(),
        }
    }
}

struct MigratableConfigV1ToV2;

impl Migration for MigratableConfigV1ToV2 {
    const FROM: u32 = 1;

    fn migrate(value: &mut Value) -> Result<(), Error> {
        if let Value::Map(map) = value {
            map.insert(
                Value::String("new_field".to_string()),
                Value::String("migrated_value".to_string()),
            );
        }
        Ok(())
    }
}

submit_migration!(MigratableConfig, MigratableConfigV1ToV2);

#[test]
fn test_migration_from_v1_to_v2() {
    let temp_dir = temp_config_dir();
    let config_path = temp_dir.path().join("migratable.toml");

    // Write a v1 config (without new_field)
    let v1_content = r#"
_version = 1
name = "old_config"
value = 50
"#;
    fs::write(&config_path, v1_content).expect("Failed to write config file");

    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<MigratableConfig>()
        .expect("Failed to register config");
    store
        .load::<MigratableConfig>()
        .expect("Failed to load config");

    let config = store
        .get::<MigratableConfig>()
        .expect("Failed to get config");

    // Original fields should be preserved
    assert_eq!(config.name, "old_config");
    assert_eq!(config.value, 50);

    // New field should have the migrated value
    assert_eq!(config.new_field, "migrated_value");

    // File should be updated with new version
    let updated_content = fs::read_to_string(&config_path).expect("Failed to read config file");
    assert!(
        updated_content.contains("_version = 2"),
        "Version should be updated to 2"
    );
}

#[test]
fn test_no_migration_needed_for_current_version() {
    let temp_dir = temp_config_dir();
    let config_path = temp_dir.path().join("migratable.toml");

    // Write a v2 config (current version)
    let v2_content = r#"
_version = 2
name = "current_config"
value = 75
new_field = "already_set"
"#;
    fs::write(&config_path, v2_content).expect("Failed to write config file");

    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<MigratableConfig>()
        .expect("Failed to register config");
    store
        .load::<MigratableConfig>()
        .expect("Failed to load config");

    let config = store
        .get::<MigratableConfig>()
        .expect("Failed to get config");

    // All fields should retain their values (no migration applied)
    assert_eq!(config.name, "current_config");
    assert_eq!(config.value, 75);
    assert_eq!(config.new_field, "already_set");
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config)]
#[config(version = 3, file_name = "multi_migration.toml")]
struct MultiMigrationConfig {
    name: String,
    timeout: u32,    // Added in v2
    max_retries: u8, // Added in v3
}

impl Default for MultiMigrationConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            timeout: 30,
            max_retries: 3,
        }
    }
}

struct MultiMigrationV1ToV2;

impl Migration for MultiMigrationV1ToV2 {
    const FROM: u32 = 1;

    fn migrate(value: &mut Value) -> Result<(), Error> {
        if let Value::Map(map) = value {
            map.insert(
                Value::String("timeout".to_string()),
                Value::U32(60), // Default timeout for migrated configs
            );
        }
        Ok(())
    }
}

struct MultiMigrationV2ToV3;

impl Migration for MultiMigrationV2ToV3 {
    const FROM: u32 = 2;

    fn migrate(value: &mut Value) -> Result<(), Error> {
        if let Value::Map(map) = value {
            map.insert(
                Value::String("max_retries".to_string()),
                Value::U8(5), // Default retries for migrated configs
            );
        }
        Ok(())
    }
}

submit_migration!(MultiMigrationConfig, MultiMigrationV1ToV2);
submit_migration!(MultiMigrationConfig, MultiMigrationV2ToV3);

#[test]
fn test_multi_step_migration_v1_to_v3() {
    let temp_dir = temp_config_dir();
    let config_path = temp_dir.path().join("multi_migration.toml");

    // Write a v1 config
    let v1_content = r#"
_version = 1
name = "legacy_config"
"#;
    fs::write(&config_path, v1_content).expect("Failed to write config file");

    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<MultiMigrationConfig>()
        .expect("Failed to register config");
    store
        .load::<MultiMigrationConfig>()
        .expect("Failed to load config");

    let config = store
        .get::<MultiMigrationConfig>()
        .expect("Failed to get config");

    // Original field preserved
    assert_eq!(config.name, "legacy_config");

    // Fields added by migrations should have migration defaults
    assert_eq!(config.timeout, 60); // Added in v1->v2
    assert_eq!(config.max_retries, 5); // Added in v2->v3

    // File should be at version 3
    let updated_content = fs::read_to_string(&config_path).expect("Failed to read config file");
    assert!(
        updated_content.contains("_version = 3"),
        "Version should be updated to 3"
    );
}

#[test]
fn test_partial_migration_v2_to_v3() {
    let temp_dir = temp_config_dir();
    let config_path = temp_dir.path().join("multi_migration.toml");

    // Write a v2 config (only needs v2->v3 migration)
    let v2_content = r#"
_version = 2
name = "partial_migration"
timeout = 120
"#;
    fs::write(&config_path, v2_content).expect("Failed to write config file");

    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<MultiMigrationConfig>()
        .expect("Failed to register config");
    store
        .load::<MultiMigrationConfig>()
        .expect("Failed to load config");

    let config = store
        .get::<MultiMigrationConfig>()
        .expect("Failed to get config");

    // Original fields preserved
    assert_eq!(config.name, "partial_migration");
    assert_eq!(config.timeout, 120); // Was already set in v2

    // Only v3 field should be migration default
    assert_eq!(config.max_retries, 5);

    // File should be at version 3
    let updated_content = fs::read_to_string(&config_path).expect("Failed to read config file");
    assert!(
        updated_content.contains("_version = 3"),
        "Version should be updated to 3"
    );
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Config)]
#[config(version = 2, file_name = "transform.toml")]
struct TransformConfig {
    // In v1 this was "hostname", renamed to "host" in v2
    host: String,
    port: u16,
}

impl Default for TransformConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
        }
    }
}

struct TransformConfigV1ToV2;

impl Migration for TransformConfigV1ToV2 {
    const FROM: u32 = 1;

    fn migrate(value: &mut Value) -> Result<(), Error> {
        if let Value::Map(map) = value {
            // Rename "hostname" to "host"
            if let Some(hostname_value) = map.remove(&Value::String("hostname".to_string())) {
                map.insert(Value::String("host".to_string()), hostname_value);
            }
        }
        Ok(())
    }
}

submit_migration!(TransformConfig, TransformConfigV1ToV2);

#[test]
fn test_migration_with_field_rename() {
    let temp_dir = temp_config_dir();
    let config_path = temp_dir.path().join("transform.toml");

    // Write a v1 config with old field name
    let v1_content = r#"
_version = 1
hostname = "myserver.example.com"
port = 443
"#;
    fs::write(&config_path, v1_content).expect("Failed to write config file");

    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<TransformConfig>()
        .expect("Failed to register config");
    store
        .load::<TransformConfig>()
        .expect("Failed to load config");

    let config = store
        .get::<TransformConfig>()
        .expect("Failed to get config");

    // Renamed field should have the old value
    assert_eq!(config.host, "myserver.example.com");
    assert_eq!(config.port, 443);

    // Verify the file has the new field name
    let updated_content = fs::read_to_string(&config_path).expect("Failed to read config file");
    assert!(
        updated_content.contains("host = "),
        "File should contain 'host' field"
    );
    assert!(
        !updated_content.contains("hostname = "),
        "File should not contain old 'hostname' field"
    );
}

#[test]
fn test_config_without_version_assumes_v1() {
    let temp_dir = temp_config_dir();
    let config_path = temp_dir.path().join("migratable.toml");

    // Write a config without _version field (should be treated as v1)
    let content = r#"
name = "no_version"
value = 100
"#;
    fs::write(&config_path, content).expect("Failed to write config file");

    let mut store = ConfigStore::init(temp_dir.path()).expect("Failed to create store");
    store
        .register::<MigratableConfig>()
        .expect("Failed to register config");
    store
        .load::<MigratableConfig>()
        .expect("Failed to load config");

    let config = store
        .get::<MigratableConfig>()
        .expect("Failed to get config");

    // Should have been migrated from v1 -> v2
    assert_eq!(config.name, "no_version");
    assert_eq!(config.value, 100);
    assert_eq!(config.new_field, "migrated_value"); // Added by migration

    // Version should now be present in file
    let updated_content = fs::read_to_string(&config_path).expect("Failed to read config file");
    assert!(
        updated_content.contains("_version = 2"),
        "Version should be set to 2 after migration"
    );
}
