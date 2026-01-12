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
//! use next_config::Config;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Debug, Default, Serialize, Deserialize, Config)]
//! #[config(version = 1, file_name = "app.toml")]
//! struct AppConfig {
//!     name: String,
//!     port: u16,
//!     debug: bool,
//! }
//! ```
//!
//! ### 2. Use the Config Store
//!
//! ```rust,no_run
//! # use next_config::Config;
//! use next_config::ConfigStore;
//! # use serde::{Serialize, Deserialize};
//! # #[derive(Debug, Default, Serialize, Deserialize, Config)]
//! # #[config(version = 1, file_name = "app.toml")]
//! # struct AppConfig { name: String, port: u16, debug: bool }
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize the store with a config directory
//!     let mut store = ConfigStore::builder()
//!         .register::<AppConfig>()?
//!         .init("./config");
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
//! use next_config::{Config, Migration, submit_migration, error::Error};
//! use serde::{Serialize, Deserialize};
//! use serde_value::Value;
//!
//! #[derive(Debug, Default, Serialize, Deserialize, Config)]
//! #[config(version = 2, file_name = "app.toml")]
//! struct AppConfig {
//!     name: String,
//!     port: u16,
//!     debug: bool,
//!     max_connections: u32,  // New field in version 2
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
//! submit_migration!(AppConfig, AppConfigV1ToV2);
//! ```
//!
//! ## Architecture
//!
//! The library uses the [`inventory`] crate for compile-time registration of config types
//! and migrations. This allows for a decentralized approach where configs can be defined
//! in different modules and automatically collected at runtime.
//!
//! - [`Config`]: Derive macro that implements all required traits and registers the config
//! - [`Config`]: The underlying trait that config structs implement
//! - [`ConfigStore`]: The central registry that manages all config instances
//! - [`Migration`]: Trait for defining schema migrations between versions
//! - [`submit_migration!`]: Macro to register a migration

mod config;
pub mod error;
mod migration;
mod store;

pub use config::Config;
pub use migration::{Migration, RegisteredMigration};
pub use store::ConfigStore;

// Re-export the derive macro
pub use next_config_macros::Config;
