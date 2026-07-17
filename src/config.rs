use serde::{Serialize, de::DeserializeOwned};

pub trait Config: Serialize + DeserializeOwned + 'static {
    const VERSION: u32;
}
