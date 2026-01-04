use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::Config;

pub struct GlobalConfig {
    inner: HashMap<TypeId, Box<dyn Any>>,
}

impl GlobalConfig {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn insert<C: Config + 'static>(&mut self, cfg: C) {
        self.inner.insert(TypeId::of::<C>(), Box::new(cfg));
    }

    pub fn get<C: Config + 'static>(&self) -> Option<&C> {
        self.inner
            .get(&TypeId::of::<C>())
            .and_then(|v| v.downcast_ref())
    }
}
