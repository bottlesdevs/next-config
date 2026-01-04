use std::path::Path;
use std::fs;

use toml::Value;

use crate::atomic::AtomicFile;
use crate::ConfigError;

pub trait Config:
    Sized + serde::Serialize + for<'de> serde::Deserialize<'de>
{
    const FILE_NAME: &'static str;
    const VERSION: u32;

    fn migrate(
        _from: u32,
        data: Value,
    ) -> Result<Value, ConfigError> {
        Ok(data)
    }

    fn to_toml_string(&self) -> Result<String, ConfigError> {
        Ok(toml::to_string(self)?)
    }

    fn load_from_str(toml_str: &str) -> Result<Self, ConfigError> {
        Ok(toml::from_str(toml_str)?)
    }

    fn save_to_file<P: AsRef<Path>>(&self, dir: P) -> Result<(), ConfigError> {
        let mut value = Value::try_from(self)?;

        let mut meta = toml::value::Table::new();
        meta.insert("version".into(), Value::Integer(Self::VERSION as i64));
        value
            .as_table_mut()
            .ok_or_else(|| ConfigError::Migration("Config must serialize as a table".into()))?
            .insert("__meta".into(), Value::Table(meta));

        let toml_str = toml::to_string_pretty(&value)?;

        let path = dir.as_ref().join(Self::FILE_NAME);
        let afile = AtomicFile::new(&path);
        afile.write(&toml_str)?;

        Ok(())
    }

    fn load_from_file<P: AsRef<Path>>(dir: P) -> Result<Self, ConfigError> {
        let path = dir.as_ref().join(Self::FILE_NAME);
        let raw = fs::read_to_string(&path)?;

        let value: Value = toml::from_str(&raw)?;

        let version = value
            .get("__meta")
            .and_then(|m| m.get("version"))
            .and_then(|v| v.as_integer())
            .unwrap_or(1) as u32;

        let migrated = if version < Self::VERSION {
            Self::migrate(version, value)?
        } else {
            value
        };

        Ok(migrated.try_into()?)
    }
}
