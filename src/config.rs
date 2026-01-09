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

pub trait Config: Default + Send + Sync + Serialize + DeserializeOwned + 'static {
    const VERSION: u32;
    const FILE_NAME: &'static str;
}

pub trait AnyConfig: Send + Sync {
    fn inner(&self) -> &dyn Any;
    fn inner_mut(&mut self) -> &mut dyn Any;

    fn load_from_dir(&mut self, conf_dir: &Path) -> Result<(), Error>;
    fn save(&self, config_dir: &Path) -> Result<(), Error>;
}

pub struct ConfigData<T>(Option<T>);

impl<T: Config> ConfigData<T> {
    fn merge_defaults(target: &mut Value) -> Result<(), Error> {
        let defaults = serde_value::to_value(T::default())?;

        if let (Value::Map(target_map), Value::Map(defaults_map)) = (target, defaults) {
            for (k, v) in defaults_map {
                target_map.entry(k).or_insert(v);
            }
        }

        Ok(())
    }

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

    fn extract_version(value: &Value) -> u32 {
        if let Value::Map(map) = value {
            for (k, v) in map {
                if let Value::String(key_str) = k {
                    if key_str == "_version" {
                        return match v {
                            Value::U8(n) => *n as u32,
                            Value::U16(n) => *n as u32,
                            Value::U32(n) => *n,
                            Value::U64(n) => *n as u32,
                            Value::I8(n) => *n as u32,
                            Value::I16(n) => *n as u32,
                            Value::I32(n) => *n as u32,
                            Value::I64(n) => *n as u32,
                            _ => 1,
                        };
                    }
                }
            }
        }
        1 // Default to version 1 if not specified
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

pub struct RegisteredConfig {
    pub config: fn() -> Box<dyn AnyConfig>,
    pub id: fn() -> TypeId,
}

impl RegisteredConfig {
    pub const fn new<T: Config>() -> Self {
        Self {
            config: || Box::new(ConfigData::<T>::default()),
            id: || TypeId::of::<T>(),
        }
    }
}

inventory::collect!(RegisteredConfig);

#[macro_export]
macro_rules! submit_config {
    ($config_type:ty) => {
        ::inventory::submit! {
            $crate::RegisteredConfig::new::<$config_type>()
        }
    };
}
