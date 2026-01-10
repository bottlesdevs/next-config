//! Configuration trait and type registration system.
//!
//! This module provides the core [`Config`] trait that all configuration types must
//! implement, along with the infrastructure for type-erased storage and serialization.
//!
//! # Overview
//!
//! The configuration system works through a combination of:
//!
//! 1. **The [`Config`] trait**: Defines the interface that all config types must implement
//! 2. **Type-erased storage**: [`AnyConfig`] and [`ConfigData<T>`] enable storing different
//!    config types in a single collection
//! 3. **Compile-time registration**: The [`Config`](crate::Config) derive macro
//!    registers config types using the [`inventory`] crate
//!
//! # Example
//!
//! ```rust
//! use next_config::Config;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Default, Serialize, Deserialize, Config)]
//! #[config(version = 1, file_name = "database.toml")]
//! struct DatabaseConfig {
//!     host: String,
//!     port: u16,
//!     max_connections: u32,
//! }
//! ```

use crate::{
    error::Error,
    migration::{MigrateFn, RegisteredMigration},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_value::Value;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    path::Path,
};

/// The main trait that all configuration types must implement.
///
/// This trait defines the contract for configuration structs, including
/// versioning information and file naming. Types implementing this trait
/// can be managed by the [`ConfigStore`](crate::ConfigStore).
///
/// # Versioning
///
/// The `VERSION` constant is used to track schema changes. When you modify
/// the structure of your config (adding/removing/renaming fields), you should:
///
/// 1. Increment the `VERSION` constant
/// 2. Optionally define and register a [`Migration`](crate::Migration) to transform old configs
///
/// # Example
///
/// ```rust
/// use next_config::Config;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct AppConfig {
///     app_name: String,
///     debug_mode: bool,
///     log_level: String,
/// }
///
/// impl Config for AppConfig {
///     /// Current schema version - increment when changing the struct
///     const VERSION: u32 = 1;
///
///     /// File will be saved as "app.toml" in the config directory
///     const FILE_NAME: &'static str = "app.toml";
/// }
///
/// impl Default for AppConfig {
///     fn default() -> Self {
///         Self {
///             app_name: "MyApp".to_string(),
///             debug_mode: false,
///             log_level: "info".to_string(),
///         }
///     }
/// }
/// ```
pub trait Config: Default + Send + Sync + Serialize + DeserializeOwned + 'static {
    /// The current schema version of this configuration.
    ///
    /// This version number is stored in the config file as `_version` and is
    /// used to determine if migrations need to be applied when loading.
    const VERSION: u32;

    /// The filename for this configuration file.
    ///
    /// This should be a simple filename (not a path) ending in `.toml`.
    /// The file will be created in the directory passed to
    /// [`ConfigStore::init`](crate::ConfigStore::init).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use next_config::Config;
    /// # use serde::{Deserialize, Serialize};
    /// # #[derive(Debug, Default, Serialize, Deserialize)]
    /// # struct MyConfig;
    /// impl Config for MyConfig {
    ///     const VERSION: u32 = 1;
    ///     const FILE_NAME: &'static str = "my_config.toml";
    /// }
    /// ```
    const FILE_NAME: &'static str;
}

/// Internal trait for type-erased configuration storage.
///
/// This trait enables the [`ConfigStore`](crate::ConfigStore) to store
/// different configuration types in a single `HashMap` while still
/// supporting type-safe access through downcasting.
///
/// This trait is not intended to be implemented directly by users.
pub trait AnyConfig: Send + Sync {
    /// Returns a reference to the inner config as `dyn Any`.
    ///
    /// Used for downcasting to the concrete config type.
    fn inner(&self) -> &dyn Any;

    /// Returns a mutable reference to the inner config as `dyn Any`.
    ///
    /// Used for downcasting to the concrete config type for updates.
    fn inner_mut(&mut self) -> &mut dyn Any;

    /// Loads the configuration from a directory.
    ///
    /// If the config file exists, it will be read and potentially migrated.
    /// If it doesn't exist, a new file with default values will be created.
    ///
    /// # Arguments
    ///
    /// * `conf_dir` - The directory containing configuration files
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML cannot be parsed
    /// - Migration fails
    /// - The config cannot be deserialized
    fn load_from_dir(&mut self, conf_dir: &Path) -> Result<(), Error>;

    /// Saves the configuration to a directory.
    ///
    /// The save operation is atomic: the config is first written to a
    /// temporary file, then renamed to the final destination.
    ///
    /// # Arguments
    ///
    /// * `config_dir` - The directory to save the configuration to
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The config cannot be serialized
    /// - The file cannot be written
    fn save(&self, config_dir: &Path) -> Result<(), Error>;
}

/// Wrapper type that holds configuration data and handles serialization.
///
/// This struct implements [`AnyConfig`] for any type that implements [`Config`],
/// providing the bridge between the type-safe config world and the type-erased
/// storage in [`ConfigStore`](crate::ConfigStore).
pub struct ConfigData<T>(Option<T>);

impl<T: Config> ConfigData<T> {
    /// Merges default values into the target value.
    ///
    /// This ensures that any fields missing from the loaded config
    /// are populated with their default values.
    ///
    /// # Arguments
    ///
    /// * `target` - The value to merge defaults into
    ///
    /// # Errors
    ///
    /// Returns an error if the default config cannot be serialized.
    fn merge_defaults(target: &mut Value) -> Result<(), Error> {
        let defaults = serde_value::to_value(T::default())?;

        if let (Value::Map(target_map), Value::Map(defaults_map)) = (target, defaults) {
            for (k, v) in defaults_map {
                target_map.entry(k).or_insert(v);
            }
        }

        Ok(())
    }

    /// Applies migrations to bring the config up to the current version.
    ///
    /// This method:
    /// 1. Extracts the current version from the value
    /// 2. Applies registered migrations in sequence
    /// 3. Updates the version field
    ///
    /// # Arguments
    ///
    /// * `value` - The config value to migrate
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if migrations were applied, `Ok(false)` if no
    /// migration was needed (config was already at current version).
    ///
    /// # Errors
    ///
    /// Returns an error if any migration function fails.
    fn migrate(&self, value: &mut Value) -> Result<bool, Error> {
        let migrations: HashMap<u32, MigrateFn> = inventory::iter::<RegisteredMigration>
            .into_iter()
            .filter_map(|migration| match (migration.id)() == TypeId::of::<T>() {
                true => Some(((migration.from)(), migration.f)),
                false => None,
            })
            .collect();

        let mut current_version = Self::extract_version(value);
        let needs_migration = current_version != T::VERSION;

        while current_version != T::VERSION {
            Self::merge_defaults(value)?;

            if let Some(migrate_fn) = migrations.get(&current_version) {
                migrate_fn(value)?;
            }

            current_version += 1;
        }

        if let Value::Map(map) = value {
            map.insert(
                Value::String("_version".to_string()),
                Value::U32(T::VERSION),
            );
        }

        Ok(needs_migration)
    }

    /// Extracts the version number from a config value.
    ///
    /// Looks for a `_version` field in the map and attempts to parse it
    /// as a number. Supports various integer types for flexibility.
    ///
    /// # Arguments
    ///
    /// * `value` - The config value to extract the version from
    ///
    /// # Returns
    ///
    /// The version number, or `1` if no version field is present (assumes
    /// legacy configs without versioning are version 1).
    fn extract_version(value: &Value) -> u32 {
        match value {
            Value::Map(map) => map
                .get(&Value::String("_version".to_string()))
                .map(|val| {
                    Value::deserialize_into(val.clone()).expect("Failed to deserialize _version")
                })
                .unwrap_or(1u32),
            _ => panic!("Expected config to serialize to a map"),
        }
    }
}

impl<T: Config> AnyConfig for ConfigData<T> {
    fn inner(&self) -> &dyn Any {
        self.0.as_ref().unwrap()
    }

    fn inner_mut(&mut self) -> &mut dyn Any {
        self.0.as_mut().unwrap()
    }

    fn load_from_dir(&mut self, conf_dir: &Path) -> Result<(), Error> {
        let fs_path = conf_dir.join(T::FILE_NAME);

        let mut value: Value = match fs_path.exists() {
            true => {
                let contents = std::fs::read_to_string(&fs_path)?;
                toml::from_str(&contents)?
            }
            false => {
                let default_value = serde_value::to_value(T::default())?;
                match default_value {
                    Value::Map(mut map) => {
                        map.insert(
                            Value::String("_version".to_string()),
                            Value::U32(T::VERSION),
                        );
                        Value::Map(map)
                    }
                    _ => {
                        return Err(Error::Deserialization(
                            serde_value::DeserializerError::Custom(
                                "Expected config to serialize to a map".to_string(),
                            ),
                        ));
                    }
                }
            }
        };
        let migrated = self.migrate(&mut value)?;

        let instance: T = serde::Deserialize::deserialize(value)?;
        self.0 = Some(instance);

        // Save the migrated config back to disk if migration occurred
        if migrated || !fs_path.exists() {
            self.save(conf_dir)?;
        }

        Ok(())
    }

    fn save(&self, config_dir: &Path) -> Result<(), Error> {
        let destination = config_dir.join(T::FILE_NAME);

        let mut config = self
            .0
            .as_ref()
            .map(serde_value::to_value)
            .ok_or("Config not loaded")
            .expect("Config not loaded")?;

        match config {
            Value::Map(ref mut map) => {
                map.insert(
                    Value::String("_version".to_string()),
                    Value::U32(T::VERSION),
                );
            }
            _ => {
                return Err(Error::Deserialization(
                    serde_value::DeserializerError::Custom(
                        "Expected config to serialize to a map".to_string(),
                    ),
                ));
            }
        };

        let toml_string = toml::to_string_pretty(&config)?;
        let temp_path = destination.with_extension("tmp");
        std::fs::write(&temp_path, toml_string)?;
        std::fs::rename(&temp_path, &destination)?;

        Ok(())
    }
}

impl<T: Config> Default for ConfigData<T> {
    fn default() -> Self {
        ConfigData(None)
    }
}

/// A descriptor for a registered configuration type.
///
/// This struct is created by the [`Config`](crate::Config) derive macro
/// and collected at runtime by the [`inventory`] crate. It provides factory
/// functions for creating config instances and identifying config types.
///
/// # Usage
///
/// You should not create this struct directly. Instead, use the `#[derive(Config)]` macro:
///
/// ```rust
/// use next_config::Config;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Default, Serialize, Deserialize, Config)]
/// #[config(version = 1, file_name = "my_config.toml")]
/// struct MyConfig {
///     value: i32,
/// }
/// // RegisteredConfig is automatically created and registered
/// ```
pub struct RegisteredConfig {
    /// Factory function that creates a new boxed config instance.
    pub config: fn() -> Box<dyn AnyConfig>,

    /// Function that returns the [`TypeId`] of the config type.
    pub id: fn() -> TypeId,
}

impl RegisteredConfig {
    /// Creates a new `RegisteredConfig` for the given config type.
    ///
    /// This is a `const fn` to allow usage in static contexts required
    /// by the [`inventory`] crate.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The config type to register (must implement [`Config`])
    pub const fn new<T: Config>() -> Self {
        Self {
            config: || Box::new(ConfigData::<T>::default()),
            id: || TypeId::of::<T>(),
        }
    }
}

// Collect all registered configs using the inventory crate
inventory::collect!(RegisteredConfig);

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_value::Value;
    use std::collections::BTreeMap;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestConfig {
        name: String,
        count: u32,
        enabled: bool,
    }

    impl Config for TestConfig {
        const VERSION: u32 = 1;
        const FILE_NAME: &'static str = "test_config.toml";
    }

    impl Default for TestConfig {
        fn default() -> Self {
            Self {
                name: "default_name".to_string(),
                count: 42,
                enabled: true,
            }
        }
    }

    #[test]
    fn test_extract_version() {
        let mut map = BTreeMap::new();
        map.insert(Value::String("_version".to_string()), Value::U32(5));
        let value = Value::Map(map);

        assert_eq!(ConfigData::<TestConfig>::extract_version(&value), 5);
    }

    #[test]
    fn test_extract_version_missing_defaults_to_1() {
        let mut map = BTreeMap::new();
        map.insert(
            Value::String("name".to_string()),
            Value::String("test".to_string()),
        );
        let value = Value::Map(map);

        assert_eq!(ConfigData::<TestConfig>::extract_version(&value), 1);
    }

    #[test]
    #[should_panic]
    fn test_extract_version_non_map_panics() {
        let value = Value::String("not a map".to_string());

        assert_eq!(ConfigData::<TestConfig>::extract_version(&value), 1);
    }

    #[test]
    #[should_panic]
    fn test_extract_version_invalid_type_panics() {
        let mut map = BTreeMap::new();
        map.insert(
            Value::String("_version".to_string()),
            Value::String("not a number".to_string()),
        );
        let value = Value::Map(map);

        assert_eq!(ConfigData::<TestConfig>::extract_version(&value), 1);
    }

    #[test]
    fn test_merge_defaults_adds_missing_fields() {
        let mut map = BTreeMap::new();
        map.insert(
            Value::String("name".to_string()),
            Value::String("custom_name".to_string()),
        );
        let mut value = Value::Map(map);

        ConfigData::<TestConfig>::merge_defaults(&mut value).unwrap();

        match value {
            Value::Map(map) => {
                // Original field preserved
                assert_eq!(
                    map.get(&Value::String("name".to_string())),
                    Some(&Value::String("custom_name".to_string()))
                );
                // Default fields added
                assert_eq!(
                    map.get(&Value::String("count".to_string())),
                    Some(&Value::U32(42))
                );
                assert_eq!(
                    map.get(&Value::String("enabled".to_string())),
                    Some(&Value::Bool(true))
                );
            }
            _ => panic!("Expected Value::Map"),
        }
    }

    #[test]
    fn test_merge_defaults_preserves_existing_fields() {
        let mut map = BTreeMap::new();
        map.insert(
            Value::String("name".to_string()),
            Value::String("custom".to_string()),
        );
        map.insert(Value::String("count".to_string()), Value::U32(999));
        map.insert(Value::String("enabled".to_string()), Value::Bool(false));
        let mut value = Value::Map(map);

        ConfigData::<TestConfig>::merge_defaults(&mut value).unwrap();

        match value {
            Value::Map(map) => {
                // All fields should retain their original values
                assert_eq!(
                    map.get(&Value::String("name".to_string())),
                    Some(&Value::String("custom".to_string()))
                );
                assert_eq!(
                    map.get(&Value::String("count".to_string())),
                    Some(&Value::U32(999))
                );
                assert_eq!(
                    map.get(&Value::String("enabled".to_string())),
                    Some(&Value::Bool(false))
                );
            }
            _ => panic!("Expected Value::Map"),
        }
    }
}
