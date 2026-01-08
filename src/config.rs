use crate::error::Error;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    any::{Any, TypeId},
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

pub struct RegisteredConfig {
    pub config: fn() -> Box<dyn AnyConfig>,
    pub id: fn() -> TypeId,
}

inventory::collect!(RegisteredConfig);
