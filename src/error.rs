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
    #[error("Config not registered: {0}")]
    UnregisteredConfig(String),
}
