use crate::{Config, error::Error};
use serde_value::Value;
use std::any::TypeId;

pub(crate) type MigrateFn = fn(&mut Value) -> Result<(), Error>;

pub trait Migration: 'static + Send + Sync {
    const FROM: u32;
    fn migrate(value: &mut Value) -> Result<(), Error>;
}

/// A registered migration descriptor.
pub struct RegisteredMigration {
    pub id: fn() -> TypeId,
    pub from: fn() -> u32,
    pub f: MigrateFn,
}

impl RegisteredMigration {
    pub const fn new<T: Config, M: Migration>() -> Self {
        Self {
            id: || TypeId::of::<T>(),
            from: || M::FROM,
            f: M::migrate,
        }
    }
}

inventory::collect!(RegisteredMigration);

#[macro_export]
macro_rules! submit_migration {
    ($config_type:ty, $migrator_type:ty) => {
        ::inventory::submit! {
            $crate::RegisteredMigration::new::<$config_type, $migrator_type>()
        }
    };
}
