//! # next-config
//!
//! A flexible, type-safe configuration system for Rust with versioning and migrations.
//!
//! This crate provides a robust way to manage application configuration files with
//! automatic versioning, schema migrations, and type-safe access patterns.
//!
//! ## Quick Start
//!
//! ### 1. Define Your Config
//!
//! ```rust
//! use next_config::{Config, ConfigStore, submit_config};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Default, Serialize, Deserialize)]
//! struct AppConfig {
//!     name: String,
//!     port: u16,
//!     debug: bool,
//! }
//!
//! impl Config for AppConfig {
//!     const VERSION: u32 = 1;
//!     const FILE_NAME: &'static str = "app.toml";
//! }
//!
//! // Register the config type at compile time
//! submit_config!(AppConfig);
//! ```
//!
//! ### 2. Use the Config Store
//!
//! ```rust,no_run
//! # use next_config::{Config, ConfigStore, submit_config};
//! # use serde::{Deserialize, Serialize};
//! # #[derive(Debug, Default, Serialize, Deserialize)]
//! # struct AppConfig { name: String, port: u16, debug: bool }
//! # impl Config for AppConfig {
//! #     const VERSION: u32 = 1;
//! #     const FILE_NAME: &'static str = "app.toml";
//! # }
//! # submit_config!(AppConfig);
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize the store with a config directory
//!     let mut store = ConfigStore::init("./config")?;
//!
//!     // Load all registered configs from disk
//!     store.load_all()?;
//!
//!     // Read config immutably
//!     let config = store.get::<AppConfig>()?;
//!     println!("App: {} running on port {}", config.name, config.port);
//!
//!     // Update config (automatically saves to disk)
//!     store.update::<AppConfig, _>(|cfg| {
//!         cfg.port = 9090;
//!         cfg.debug = true;
//!         Ok(())
//!     })?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Migrations
//!
//! When you need to change your config schema, increment the version and define a migration:
//!
//! ```rust
//! use next_config::{Config, Migration, submit_config, submit_migration, error::Error};
//! use serde::{Deserialize, Serialize};
//! use serde_value::Value;
//!
//! #[derive(Debug, Default, Serialize, Deserialize)]
//! struct AppConfig {
//!     name: String,
//!     port: u16,
//!     debug: bool,
//!     max_connections: u32,  // New field in version 2
//! }
//!
//! impl Config for AppConfig {
//!     const VERSION: u32 = 2;  // Incremented from 1
//!     const FILE_NAME: &'static str = "app.toml";
//! }
//!
//! // Define the migration from version 1 to 2
//! struct AppConfigV1ToV2;
//!
//! impl Migration for AppConfigV1ToV2 {
//!     const FROM: u32 = 1;
//!
//!     fn migrate(value: &mut Value) -> Result<(), Error> {
//!         // Add the new field with a default value
//!         if let Value::Map(map) = value {
//!             map.insert(
//!                 Value::String("max_connections".to_string()),
//!                 Value::U32(100),
//!             );
//!         }
//!         Ok(())
//!     }
//! }
//!
//! submit_config!(AppConfig);
//! submit_migration!(AppConfig, AppConfigV1ToV2);
//! ```
//!
//! ## Architecture
//!
//! The library uses the [`inventory`] crate for compile-time registration of config types
//! and migrations. This allows for a decentralized approach where configs can be defined
//! in different modules and automatically collected at runtime.
//!
//! - [`Config`]: The main trait that config structs must implement
//! - [`ConfigStore`]: The central registry that manages all config instances
//! - [`Migration`]: Trait for defining schema migrations between versions
//! - [`submit_config!`]: Macro to register a config type
//! - [`submit_migration!`]: Macro to register a migration

mod config;
pub mod error;
mod migration;
mod store;

pub use config::{Config, RegisteredConfig};
pub use migration::{Migration, RegisteredMigration};
pub use store::ConfigStore;

#[cfg(test)]
mod tests {}
