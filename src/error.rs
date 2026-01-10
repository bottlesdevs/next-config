use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization: {0}")]
    Serialization(#[from] serde_value::SerializerError),

    #[error("Deserialization: {0}")]
    Deserialization(#[from] serde_value::DeserializerError),

    #[error("TOML Serialization: {0}")]
    TomlSerialization(#[from] toml::ser::Error),

    #[error("TOML Deserialization: {0}")]
    TomlDeserialization(#[from] toml::de::Error),

    /// Attempted to access a configuration type that was not registered.
    ///
    /// This error is returned when calling [`ConfigStore::get`](crate::ConfigStore::get),
    /// [`ConfigStore::load`](crate::ConfigStore::load), or
    /// [`ConfigStore::update`](crate::ConfigStore::update) with a config type
    /// that was not registered using the [`#[derive(Config)]`](crate::Config) macro.
    ///
    /// The contained string is the `FILE_NAME` of the unregistered config type.
    ///
    /// # How to Fix
    ///
    /// Make sure to register your config type using the derive macro:
    ///
    /// ```rust
    /// use next_config::Config;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Default, Serialize, Deserialize, Config)]
    /// #[config(version = 1, file_name = "my_config.toml")]
    /// struct MyConfig {
    ///     field: String,
    /// }
    /// // Config is now automatically registered!
    /// ```
    #[error("Config not registered: {0}")]
    UnregisteredConfig(String),
}
