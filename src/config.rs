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

pub struct ConfigData<T> {
    data: Option<T>,
    migrations: HashMap<u32, MigrateFn>,
}

impl<T: Config> AnyConfig for ConfigData<T> {
    fn inner(&self) -> &dyn Any {
        self.data.as_ref().unwrap()
    }

    fn inner_mut(&mut self) -> &mut dyn Any {
        self.data.as_mut().unwrap()
    }

    fn load_from_dir(&mut self, conf_dir: &Path) -> Result<(), Error> {
        let fs_path = conf_dir.join(T::FILE_NAME);

        let value: Value = match fs_path.exists() {
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

        let instance: T = serde::Deserialize::deserialize(value)?;
        self.data = Some(instance);

        inventory::iter::<RegisteredMigration>
            .into_iter()
            .filter(|migration| (migration.id)() == TypeId::of::<T>())
            .for_each(|migration| {
                self.migrations.insert((migration.from)(), migration.f);
            });

        if !fs_path.exists() {
            self.save(conf_dir)?;
        }

        Ok(())
    }

    fn save(&self, config_dir: &Path) -> Result<(), Error> {
        let destination = config_dir.join(T::FILE_NAME);

        let mut config = self
            .data
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
        ConfigData {
            data: None,
            migrations: HashMap::new(),
        }
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
