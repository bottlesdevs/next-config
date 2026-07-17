# next-config

Typed TOML loading and saving with explicit schema versions and sequential migrations.

```rust
use next_config::{Config, load, save};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Config)]
#[config(version = 1)]
struct AppConfig {
    name: String,
    port: u16,
}

let path = "config/app.toml";
let config = AppConfig { name: "demo".into(), port: 8080 };
save(path, &config)?;
let config = load::<AppConfig>(path)?;
# Ok::<(), next_config::error::Error>(())
```

The path belongs to the caller. Files store a flat `_version` field; there is no
global store or type registration.

For a schema update, increment `#[config(version = ...)]`, implement one
`Migration` for each version step, and register each step with
`submit_migration!`. `load` applies the chain in order and persists the migrated
file.
