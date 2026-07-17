use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization: {0}")]
    Serialization(#[from] serde_value::SerializerError),
    #[error("deserialization: {0}")]
    Deserialization(#[from] serde_value::DeserializerError),
    #[error("TOML serialization: {0}")]
    TomlSerialization(#[from] toml::ser::Error),
    #[error("TOML deserialization: {0}")]
    TomlDeserialization(#[from] toml::de::Error),
    #[error("configuration root must be a table")]
    RootNotTable,
    #[error("configuration version {found} is newer than supported version {supported}")]
    UnsupportedVersion { found: u32, supported: u32 },
    #[error("missing migration from version {0}")]
    MissingMigration(u32),
    #[error("more than one migration starts at version {0}")]
    DuplicateMigration(u32),
}
