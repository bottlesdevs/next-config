pub mod config;
pub mod error;
pub mod global;
pub mod atomic;

pub use config::Config;
pub use error::ConfigError;
pub use global::GlobalConfig;

// re-export macro
pub use next_config_macro::config;
