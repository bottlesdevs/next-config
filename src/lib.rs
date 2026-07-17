mod config;
pub mod error;
mod migration;

use std::{any::TypeId, fs, path::Path};

use serde_value::Value;

pub use config::Config;
pub use migration::{Migration, RegisteredMigration};
pub use next_config_macros::Config;

use error::Error;

const VERSION_KEY: &str = "_version";

pub fn load<T: Config>(path: impl AsRef<Path>) -> Result<T, Error> {
    let path = path.as_ref();
    let mut value: Value = toml::from_str(&fs::read_to_string(path)?)?;
    let original_version = version(&value)?;
    let mut current = original_version.unwrap_or(1);
    if current > T::VERSION {
        return Err(Error::UnsupportedVersion {
            found: current,
            supported: T::VERSION,
        });
    }

    while current < T::VERSION {
        let mut migrations =
            inventory::iter::<RegisteredMigration>
                .into_iter()
                .filter(|migration| {
                    (migration.id)() == TypeId::of::<T>() && (migration.from)() == current
                });
        let migration = migrations.next().ok_or(Error::MissingMigration(current))?;
        if migrations.next().is_some() {
            return Err(Error::DuplicateMigration(current));
        }
        (migration.f)(&mut value)?;
        current += 1;
    }

    remove_version(&mut value)?;
    let config = T::deserialize(value)?;
    if original_version != Some(T::VERSION) {
        save(path, &config)?;
    }
    Ok(config)
}

pub fn save<T: Config>(path: impl AsRef<Path>, config: &T) -> Result<(), Error> {
    let path = path.as_ref();
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let mut value = serde_value::to_value(config)?;
    set_version(&mut value, T::VERSION)?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, toml::to_string_pretty(&value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn version(value: &Value) -> Result<Option<u32>, Error> {
    let Value::Map(map) = value else {
        return Err(Error::RootNotTable);
    };
    map.get(&Value::String(VERSION_KEY.into()))
        .cloned()
        .map(Value::deserialize_into)
        .transpose()
        .map_err(Into::into)
}

fn set_version(value: &mut Value, version: u32) -> Result<(), Error> {
    let Value::Map(map) = value else {
        return Err(Error::RootNotTable);
    };
    map.insert(Value::String(VERSION_KEY.into()), Value::U32(version));
    Ok(())
}

fn remove_version(value: &mut Value) -> Result<(), Error> {
    let Value::Map(map) = value else {
        return Err(Error::RootNotTable);
    };
    map.remove(&Value::String(VERSION_KEY.into()));
    Ok(())
}

#[doc(hidden)]
pub mod __private {
    pub use inventory;
}
