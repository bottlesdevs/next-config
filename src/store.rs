//! Configuration store for managing multiple config types.
//!
//! This module provides the [`ConfigStore`] type, which is the main entry point
//! for loading, accessing, and updating configuration files. The store manages
//! all registered configuration types and handles their lifecycle.
//!
//! # Overview
//!
//! The [`ConfigStore`] acts as a central registry that:
//!
//! - Collects all config types registered with [`Config`](crate::Config) derive macro
//! - Loads config files from a specified directory
//! - Provides type-safe access to configuration values
//! - Handles updates and automatic persistence
//!
//! # Example
//!
//! ```rust,no_run
//! use next_config::{Config, ConfigStore};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Default, Serialize, Deserialize, Config)]
//! #[config(version = 1, file_name = "app.toml")]
//! struct AppConfig {
//!     name: String,
//!     debug: bool,
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a store pointing to the config directory
//!     let mut store = ConfigStore::init("./config")?;
//!
//!     // Load all registered configs
//!     store.load_all()?;
//!
//!     // Access config values
//!     let app_config = store.get::<AppConfig>()?;
//!     println!("App name: {}", app_config.name);
//!
//!     // Update config (automatically persists to disk)
//!     store.update::<AppConfig, _>(|cfg| {
//!         cfg.debug = true;
//!         Ok(())
//!     })?;
//!
//!     Ok(())
//! }
//! ```
use std::{
    any::TypeId,
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    RegisteredConfig,
    config::{AnyConfig, Config},
    error::Error,
};

/// The central configuration store that manages all registered config types.
///
/// `ConfigStore` is responsible for:
///
/// - Discovering and instantiating all registered config types
/// - Loading configuration files from disk
/// - Providing type-safe read access to configurations
/// - Handling updates with automatic persistence
/// - Applying migrations when loading outdated config files
///
/// # Lifecycle
///
/// A typical usage pattern is:
///
/// 1. **Initialize**: Create a store with [`init`](ConfigStore::init)
/// 2. **Load**: Load configs with [`load_all`](ConfigStore::load_all) or [`load`](ConfigStore::load)
/// 3. **Access**: Read configs with [`get`](ConfigStore::get)
/// 4. **Update**: Modify configs with [`update`](ConfigStore::update)
///
/// # Example
///
/// ```rust,no_run
/// use next_config::{Config, ConfigStore};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Default, Serialize, Deserialize, Config)]
/// #[config(version = 1, file_name = "database.toml")]
/// struct DatabaseConfig {
///     host: String,
///     port: u16,
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut store = ConfigStore::init("/etc/myapp")?;
/// store.load_all()?;
///
/// let db_config = store.get::<DatabaseConfig>()?;
/// println!("Connecting to {}:{}", db_config.host, db_config.port);
/// # Ok(())
/// # }
/// ```
pub struct ConfigStore {
    /// The directory where configuration files are stored.
    conf_dir: PathBuf,

    /// Map from config type IDs to their type-erased instances.
    configs: HashMap<TypeId, Box<dyn AnyConfig>>,
}

impl ConfigStore {
    /// Creates a new configuration store with the specified config directory.
    ///
    /// This method initializes the store by collecting all config types that
    /// were registered using [`submit_config!`](crate::submit_config). The
    /// configs are not loaded from disk at this point—you must call
    /// [`load_all`](ConfigStore::load_all) or [`load`](ConfigStore::load) to
    /// actually read the configuration files.
    ///
    /// # Arguments
    ///
    /// * `conf_dir` - The directory path where configuration files are stored.
    ///   This can be an absolute or relative path.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use next_config::ConfigStore;
    ///
    /// // Using a relative path
    /// let store = ConfigStore::init("./config")?;
    ///
    /// // Using an absolute path
    /// let store = ConfigStore::init("/etc/myapp")?;
    ///
    /// // Using a PathBuf
    /// use std::path::PathBuf;
    /// let path = PathBuf::from("config");
    /// let store = ConfigStore::init(&path)?;
    /// # Ok::<(), next_config::error::Error>(())
    /// ```
    ///
    /// # Note
    ///
    /// The directory does not need to exist when calling `init`. If it doesn't
    /// exist, loading will fail unless the directory is created before loading.
    pub fn init(conf_dir: impl AsRef<Path>) -> Result<Self, Error> {
        let mut configs = HashMap::new();
        for registration in inventory::iter::<RegisteredConfig> {
            configs.insert((registration.id)(), (registration.config)());
        }

        Ok(Self {
            configs,
            conf_dir: conf_dir.as_ref().to_path_buf(),
        })
    }

    /// Returns a reference to a loaded configuration.
    ///
    /// This method retrieves a configuration that was previously loaded using
    /// [`load`](ConfigStore::load) or [`load_all`](ConfigStore::load_all).
    ///
    /// # Type Parameters
    ///
    /// * `T` - The configuration type to retrieve. Must implement [`Config`]
    ///   and have been registered with [`submit_config!`](crate::submit_config).
    ///
    /// # Returns
    ///
    /// Returns `Ok(&T)` with a reference to the configuration, or an error if:
    ///
    /// - The config type was not registered (returns [`Error::UnregisteredConfig`])
    ///
    /// # Panics
    ///
    /// Panics if the config was registered but not yet loaded. Always call
    /// [`load`](ConfigStore::load) or [`load_all`](ConfigStore::load_all)
    /// before accessing configs.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use next_config::{Config, ConfigStore};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Default, Serialize, Deserialize, Config)]
    /// #[config(version = 1, file_name = "app.toml")]
    /// struct AppConfig {
    ///     name: String,
    /// }
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut store = ConfigStore::init("./config")?;
    /// store.load_all()?;  // Must load before get!
    ///
    /// let config = store.get::<AppConfig>()?;
    /// println!("App name: {}", config.name);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get<T: Config>(&self) -> Result<&T, Error> {
        let type_id = TypeId::of::<T>();
        let config = self
            .configs
            .get(&type_id)
            .ok_or(Error::UnregisteredConfig(T::FILE_NAME.to_string()))?;

        let data = config
            .inner()
            .downcast_ref::<T>()
            .expect("Failed to downcast config data");

        Ok(data)
    }

    /// Loads a specific configuration type from disk.
    ///
    /// This method reads the configuration file for type `T` from the config
    /// directory. If the file doesn't exist, it creates a new one with default
    /// values. If the file exists but is an older version, migrations are
    /// applied automatically.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The configuration type to load. Must implement [`Config`](crate::Config) and
    ///   have been registered with [`#[derive(Config)]`](crate::Config).
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if:
    ///
    /// - The config type was not registered ([`Error::UnregisteredConfig`])
    /// - The file cannot be read ([`Error::Io`])
    /// - The TOML is invalid ([`Error::TomlDeserialization`])
    /// - Migration fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use next_config::{Config, ConfigStore};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Default, Serialize, Deserialize, Config)]
    /// #[config(version = 1, file_name = "database.toml")]
    /// struct DatabaseConfig {
    ///     host: String,
    /// }
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut store = ConfigStore::init("./config")?;
    ///
    /// // Load only the database config
    /// store.load::<DatabaseConfig>()?;
    ///
    /// let db_config = store.get::<DatabaseConfig>()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    ///
    /// - [`load_all`](ConfigStore::load_all) - Load all registered configs at once
    pub fn load<T: Config>(&mut self) -> Result<(), Error> {
        let type_id = TypeId::of::<T>();
        let config = self
            .configs
            .get_mut(&type_id)
            .ok_or(Error::UnregisteredConfig(T::FILE_NAME.to_string()))?;

        config.load_from_dir(&self.conf_dir)
    }

    /// Loads all registered configuration types from disk.
    ///
    /// This method iterates over all config types registered with
    /// [`submit_config!`](crate::submit_config) and loads each one. For each
    /// config, if the file doesn't exist, a new one with default values is
    /// created. If a file exists but is an older version, migrations are
    /// applied automatically.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all configs loaded successfully, or the first error
    /// encountered. Note that if an error occurs, some configs may have been
    /// loaded while others were not.
    ///
    /// # Errors
    ///
    /// This method can return any of the errors that [`load`](ConfigStore::load)
    /// can return:
    ///
    /// - [`Error::Io`] - File system errors
    /// - [`Error::TomlDeserialization`] - Invalid TOML syntax
    /// - Other deserialization or migration errors
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use next_config::ConfigStore;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut store = ConfigStore::init("./config")?;
    ///
    /// // Load all registered configs
    /// store.load_all()?;
    ///
    /// // Now all configs are available via get()
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    ///
    /// - [`load`](ConfigStore::load) - Load a specific config type
    pub fn load_all(&mut self) -> Result<(), Error> {
        for (_, config) in self.configs.iter_mut() {
            config.load_from_dir(&self.conf_dir)?
        }

        Ok(())
    }

    /// Updates a configuration and automatically saves it to disk.
    ///
    /// This method provides a way to modify a configuration through a closure.
    /// After the closure returns successfully, the updated configuration is
    /// immediately persisted to disk using atomic writes.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The configuration type to update. Must implement [`Config`](crate::Config)
    /// * `F` - The update function type.
    ///
    /// # Arguments
    ///
    /// * `f` - A closure that receives a mutable reference to the configuration.
    ///   The closure can modify the config and should return `Ok(())` on success
    ///   or an error to abort the update.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the update and save succeeded, or an error if:
    ///
    /// - The config type was not registered ([`Error::UnregisteredConfig`])
    /// - The closure returned an error
    /// - Saving to disk failed ([`Error::Io`], [`Error::TomlSerialization`])
    ///
    /// # Atomicity
    ///
    /// If the closure returns an error, the configuration is left in its
    /// original state (the changes made by the closure are discarded on the
    /// next load since they weren't saved).
    ///
    /// The save operation is atomic: changes are written to a temporary file
    /// first, then renamed to the final location. This prevents corruption
    /// if the process is interrupted during the write.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use next_config::{Config, ConfigStore, error::Error};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Default, Serialize, Deserialize, Config)]
    /// #[config(version = 1, file_name = "server.toml")]
    /// struct ServerConfig {
    ///     port: u16,
    ///     max_connections: u32,
    /// }
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut store = ConfigStore::init("./config")?;
    /// store.load_all()?;
    ///
    /// // Simple update
    /// store.update::<ServerConfig, _>(|cfg| {
    ///     cfg.port = 8080;
    ///     cfg.max_connections = 1000;
    ///     Ok(())
    /// })?;
    ///
    /// // Update with validation
    /// store.update::<ServerConfig, _>(|cfg| {
    ///     if cfg.port < 1024 {
    ///         // Return an error to abort the update
    ///         return Err(Error::Io(std::io::Error::new(
    ///             std::io::ErrorKind::InvalidInput,
    ///             "Port must be >= 1024"
    ///         )));
    ///     }
    ///     cfg.port = 9000;
    ///     Ok(())
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the config was registered but not yet loaded. Always call
    /// [`load`](ConfigStore::load) or [`load_all`](ConfigStore::load_all)
    /// before updating configs.
    pub fn update<T: Config, F>(&mut self, f: F) -> Result<(), Error>
    where
        F: FnOnce(&mut T) -> Result<(), Error>,
    {
        let type_id = TypeId::of::<T>();

        let config = self
            .configs
            .get_mut(&type_id)
            .ok_or(Error::UnregisteredConfig(T::FILE_NAME.to_string()))?;

        let inner = config
            .inner_mut()
            .downcast_mut::<T>()
            .expect("Type mismatch in registry");

        f(inner)?;

        config.save(&self.conf_dir)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::TempDir;

    #[derive(Debug, Default, Serialize, Deserialize)]
    struct StoreTestConfig {
        name: String,
        value: u32,
    }

    impl Config for StoreTestConfig {
        const VERSION: u32 = 1;
        const FILE_NAME: &'static str = "store_test.toml";
    }

    // Manual inventory submission
    ::inventory::submit! {
        crate::RegisteredConfig::new::<StoreTestConfig>()
    }

    fn temp_config_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp directory")
    }

    #[test]
    fn test_config_store_initialization() {
        let temp_dir = temp_config_dir();
        let store = ConfigStore::init(temp_dir.path());
        assert!(store.is_ok(), "ConfigStore should initialize successfully");
    }

    #[test]
    fn test_init_collects_registered_configs() {
        let temp_dir = temp_config_dir();
        let store = ConfigStore::init(temp_dir.path()).unwrap();

        // Should have at least the StoreTestConfig registered
        let type_id = TypeId::of::<StoreTestConfig>();
        assert!(
            store.configs.contains_key(&type_id),
            "Store should contain registered config"
        );
    }

    #[derive(Debug, Default, Serialize, Deserialize)]
    struct UnregisteredStoreConfig {
        field: String,
    }

    impl Config for UnregisteredStoreConfig {
        const VERSION: u32 = 1;
        const FILE_NAME: &'static str = "unregistered_store.toml";
    }

    #[test]
    fn test_get_unregistered_returns_error() {
        let temp_dir = temp_config_dir();
        let store = ConfigStore::init(temp_dir.path()).unwrap();

        let result = store.get::<UnregisteredStoreConfig>();
        assert!(matches!(result, Err(Error::UnregisteredConfig(_))));
    }

    #[test]
    fn test_configs_map_uses_type_id_as_key() {
        let temp_dir = temp_config_dir();
        let store = ConfigStore::init(temp_dir.path()).unwrap();

        // Verify that TypeId is used as key
        let type_id = TypeId::of::<StoreTestConfig>();
        assert!(store.configs.get(&type_id).is_some());
    }
}
