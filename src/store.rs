use std::{
    any::TypeId,
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{Config, RegisteredConfig, config::AnyConfig, error::Error};

pub struct ConfigStore {
    conf_dir: PathBuf,
    configs: HashMap<TypeId, Box<dyn AnyConfig>>,
}

impl ConfigStore {
    /// Initialize the config store with a custom base directory.
    pub fn init(conf_dir: impl AsRef<Path>) -> Result<Self, Error> {
        let mut configs = HashMap::new();
        for registration in inventory::iter::<RegisteredConfig> {
            configs.insert((registration.id)(), (registration.config)());
        }

        Ok(Self {
            configs,
            conf_dir: conf_dir.as_ref().to_path_buf(),
        })
    }

    pub fn get<T: Config>(&self) -> Result<&T, Error> {
        let type_id = TypeId::of::<T>();
        let config = self
            .configs
            .get(&type_id)
            .ok_or(Error::UnregisteredConfig(T::FILE_NAME.to_string()))?;

        let data = config
            .inner()
            .downcast_ref::<T>()
            .expect("Failed to downcast config data");

        Ok(data)
    }

    pub fn load<T: Config>(&mut self) -> Result<(), Error> {
        let type_id = TypeId::of::<T>();
        let config = self
            .configs
            .get_mut(&type_id)
            .ok_or(Error::UnregisteredConfig(T::FILE_NAME.to_string()))?;

        config.load_from_dir(&self.conf_dir)
    }

    pub fn load_all(&mut self) -> Result<(), Error> {
        for (_, config) in self.configs.iter_mut() {
            config.load_from_dir(&self.conf_dir)?
        }

        Ok(())
    }

    pub fn update<T: Config, F>(&mut self, f: F) -> Result<(), Error>
    where
        F: FnOnce(&mut T) -> Result<(), Error>,
    {
        let type_id = TypeId::of::<T>();

        let config = self
            .configs
            .get_mut(&type_id)
            .ok_or(Error::UnregisteredConfig(T::FILE_NAME.to_string()))?;

        let inner = config
            .inner_mut()
            .downcast_mut::<T>()
            .expect("Type mismatch in registry");

        f(inner)?;

        config.save(&self.conf_dir)?;

        Ok(())
    }
}
