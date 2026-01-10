//! Migration system for configuration schema evolution.
//!
//! This module provides the infrastructure for migrating configuration files
//! between different schema versions. When your configuration structure changes
//! (adding fields, removing fields, renaming fields, or changing types), you
//! define migrations to transform old config files to the new format.
//!
//! # Overview
//!
//! The migration system consists of:
//!
//! 1. **The [`Migration`] trait**: Defines a single migration step from one version to the next
//! 2. **[`RegisteredMigration`]**: A descriptor that links migrations to config types
//! 3. **[`submit_migration!`]**: A macro to register migrations at compile time
//!
//! # How Migrations Work
//!
//! When a config file is loaded:
//!
//! 1. The `_version` field is read from the file (defaults to `1` if missing)
//! 2. If the version is less than the config's current `VERSION`, migrations are applied
//! 3. Migrations are applied sequentially: v1→v2, v2→v3, etc.
//! 4. Default values are merged in for any missing fields
//! 5. The migrated config is saved back to disk with the new version
//!
//! # Example
//!
//! Consider a config that evolved from v1 to v3:
//!
//! ```rust
//! use next_config::{Config, Migration, submit_migration, error::Error};
//! use serde::{Deserialize, Serialize};
//! use serde_value::Value;
//!
//! // Version 3 of the config (current)
//! #[derive(Debug, Default, Serialize, Deserialize, Config)]
//! #[config(version = 3, file_name = "app.toml")]
//! struct AppConfig {
//!     name: String,
//!     port: u16,
//!     timeout_seconds: u32,  // Added in v2
//!     max_retries: u8,       // Added in v3
//! }
//!
//! // Migration from v1 to v2: add timeout_seconds
//! struct AppConfigV1ToV2;
//!
//! impl Migration for AppConfigV1ToV2 {
//!     const FROM: u32 = 1;
//!
//!     fn migrate(value: &mut Value) -> Result<(), Error> {
//!         if let Value::Map(map) = value {
//!             map.insert(
//!                 Value::String("timeout_seconds".to_string()),
//!                 Value::U32(30),  // Default timeout
//!             );
//!         }
//!         Ok(())
//!     }
//! }
//!
//! // Migration from v2 to v3: add max_retries
//! struct AppConfigV2ToV3;
//!
//! impl Migration for AppConfigV2ToV3 {
//!     const FROM: u32 = 2;
//!
//!     fn migrate(value: &mut Value) -> Result<(), Error> {
//!         if let Value::Map(map) = value {
//!             map.insert(
//!                 Value::String("max_retries".to_string()),
//!                 Value::U8(3),  // Default retries
//!             );
//!         }
//!         Ok(())
//!     }
//! }
//!
//! submit_migration!(AppConfig, AppConfigV1ToV2);
//! submit_migration!(AppConfig, AppConfigV2ToV3);
//! ```

use crate::{config::Config, error::Error};
use serde_value::Value;
use std::any::TypeId;

pub(crate) type MigrateFn = fn(&mut Value) -> Result<(), Error>;

/// Trait for defining configuration migrations.
///
/// Implement this trait to define how a configuration should be transformed
/// from one version to the next. Each migration handles a single version
/// upgrade.
///
/// # Example
///
/// ```rust
/// use next_config::{Migration, error::Error};
/// use serde_value::Value;
///
/// /// Migrates config from v1 to v2 by adding a "new_field" with default value
/// struct MyConfigV1ToV2;
///
/// impl Migration for MyConfigV1ToV2 {
///     const FROM: u32 = 1;
///
///     fn migrate(value: &mut Value) -> Result<(), Error> {
///         if let Value::Map(map) = value {
///             // Add new field with default value
///             map.insert(
///                 Value::String("new_field".to_string()),
///                 Value::Bool(true),
///             );
///
///             // You can also modify existing fields
///             // For example, rename "old_name" to "new_name":
///             if let Some(old_value) = map.remove(&Value::String("old_name".to_string())) {
///                 map.insert(Value::String("new_name".to_string()), old_value);
///             }
///         }
///         Ok(())
///     }
/// }
/// ```
pub trait Migration: 'static + Send + Sync {
    /// The version number this migration upgrades from.
    ///
    /// For example, if `FROM = 1`, this migration transforms a v1 config
    /// into a v2 config.
    ///
    /// # Guidelines
    ///
    /// - Migrations must be sequential: you need a migration for each version step
    /// - If your config is at v1 and you want to go to v3, you need both
    ///   a v1→v2 migration and a v2→v3 migration
    const FROM: u32;

    /// Performs the migration, transforming the value in place.
    ///
    /// This function receives the raw config value parsed from TOML and should
    /// modify it to conform to the new version's schema.
    ///
    /// # Arguments
    ///
    /// * `value` - The config value to transform. This is typically a
    ///   [`Value::Map`] representing the TOML table.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the migration was successful
    /// - `Err(Error)` if the migration failed (e.g., missing required data)
    ///
    /// # Panics
    ///
    /// This function should not panic. Use proper error handling and return
    /// an `Err` variant if something goes wrong.
    fn migrate(value: &mut Value) -> Result<(), Error>;
}

/// A descriptor for a registered migration.
///
/// This struct is created by the [`submit_migration!`](crate::submit_migration) macro and collected
/// at runtime by the [`inventory`] crate. It links a migration implementation
/// to a specific configuration type.
///
/// # Usage
///
/// You should not create this struct directly. Instead, use the
/// `submit_migration!` macro:
///
/// ```rust
/// use next_config::{Config, Migration, submit_migration, error::Error};
/// use serde::{Deserialize, Serialize};
/// use serde_value::Value;
///
/// #[derive(Debug, Default, Serialize, Deserialize, Config)]
/// #[config(version = 2, file_name = "my_config.toml")]
/// struct MyConfig {
///     field: String,
///     new_field: u32,  // Added in v2
/// }
///
/// struct MyConfigV1ToV2;
///
/// impl Migration for MyConfigV1ToV2 {
///     const FROM: u32 = 1;
///
///     fn migrate(value: &mut Value) -> Result<(), Error> {
///         if let Value::Map(map) = value {
///             map.insert(
///                 Value::String("new_field".to_string()),
///                 Value::U32(0),
///             );
///         }
///         Ok(())
///     }
/// }
///
/// // This creates and registers a RegisteredMigration
/// submit_migration!(MyConfig, MyConfigV1ToV2);
/// ```
pub struct RegisteredMigration {
    /// Function returning the [`TypeId`] of the config type this migration applies to.
    pub id: fn() -> TypeId,

    /// Function returning the version number this migration upgrades from.
    pub from: fn() -> u32,

    /// The migration function that transforms the config value.
    pub f: MigrateFn,
}

impl RegisteredMigration {
    /// Creates a new `RegisteredMigration` for the given config and migration types.
    ///
    /// This is a `const fn` to allow usage in static contexts required
    /// by the [`inventory`] crate.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The config type this migration applies to (must implement [`Config`])
    /// * `M` - The migration type (must implement [`Migration`])
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Typically called by the submit_migration! macro
    /// let registration = RegisteredMigration::new::<MyConfig, MyConfigV1ToV2>();
    /// ```
    pub const fn new<T: Config, M: Migration>() -> Self {
        Self {
            id: || TypeId::of::<T>(),
            from: || M::FROM,
            f: M::migrate,
        }
    }
}

// Collect all registered migrations using the inventory crate
inventory::collect!(RegisteredMigration);

/// Registers a migration for a configuration type.
///
/// This macro associates a [`Migration`] implementation with a specific
/// [`Config`] type, enabling automatic migration when loading outdated
/// config files.
///
/// # Arguments
///
/// * `$config_type` - The config type this migration applies to
/// * `$migrator_type` - The type implementing [`Migration`]
///
/// # Example
///
/// ```rust
/// use next_config::{Config, Migration, submit_migration, error::Error};
/// use serde::{Deserialize, Serialize};
/// use serde_value::Value;
///
/// #[derive(Debug, Default, Serialize, Deserialize, Config)]
/// #[config(version = 2, file_name = "network.toml")]
/// struct NetworkConfig {
///     host: String,
///     port: u16,
///     use_tls: bool,  // Added in v2
/// }
///
/// struct NetworkConfigV1ToV2;
///
/// impl Migration for NetworkConfigV1ToV2 {
///     const FROM: u32 = 1;
///
///     fn migrate(value: &mut Value) -> Result<(), Error> {
///         if let Value::Map(map) = value {
///             // Default to TLS enabled for security
///             map.insert(
///                 Value::String("use_tls".to_string()),
///                 Value::Bool(true),
///             );
///         }
///         Ok(())
///     }
/// }
///
/// submit_migration!(NetworkConfig, NetworkConfigV1ToV2);
/// ```
///
/// # Multiple Migrations
///
/// Register multiple migrations for multi-step upgrades:
///
/// ```rust,ignore
/// submit_migration!(MyConfig, MyConfigV1ToV2);
/// submit_migration!(MyConfig, MyConfigV2ToV3);
/// submit_migration!(MyConfig, MyConfigV3ToV4);
/// ```
///
/// # Note
///
/// Like [`#[derive(Config)]`](crate::Config), this macro should be called
/// at module scope to ensure the registration happens at program startup.
#[macro_export]
macro_rules! submit_migration {
    ($config_type:ty, $migrator_type:ty) => {
        ::inventory::submit! {
            $crate::RegisteredMigration::new::<$config_type, $migrator_type>()
        }
    };
}
