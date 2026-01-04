use next_config::{Config, config};
use std::fs;
use tempfile::tempdir;

#[config(name = "app_preferences")]
struct AppPreferences {
    theme: String,
    auto_save: bool,
    max_items: u32,
}

#[test]
fn test_load_and_save_preferences() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppPreferences {
        theme: "dark".into(),
        auto_save: true,
        max_items: 50,
    };

    let config_dir_path = tempdir()?;

    config.save_to_file(&config_dir_path)?;
    let loaded = AppPreferences::load_from_file(&config_dir_path)?;

    assert_eq!(config.theme, loaded.theme);
    assert_eq!(config.auto_save, loaded.auto_save);
    assert_eq!(config.max_items, loaded.max_items);

    fs::remove_dir_all(&config_dir_path)?; // Clean up
    Ok(())
}

#[test]
fn test_load_preferences_from_str() -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = r#"
theme = "light"
auto_save = false
max_items = 100
"#;

    let config = AppPreferences::load_from_str(toml_str)?;

    assert_eq!(config.theme, "light");
    assert_eq!(config.auto_save, false);
    assert_eq!(config.max_items, 100);

    Ok(())
}

#[test]
fn test_preferences_to_toml_string() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppPreferences {
        theme: "dark".into(),
        auto_save: true,
        max_items: 25,
    };

    let toml = config.to_toml_string()?;
    assert!(toml.contains("theme = \"dark\""));
    assert!(toml.contains("auto_save = true"));
    assert!(toml.contains("max_items = 25"));

    Ok(())
}

#[test]
fn test_derive_preferences_with_serde() -> Result<(), Box<dyn std::error::Error>> {
    #[config]
    struct TestPreferences {
        volume: u8,
    }

    let config = TestPreferences { volume: 75 };
    let toml = config.to_toml_string()?;
    assert!(toml.contains("volume = 75"));

    let loaded = TestPreferences::load_from_str(&toml)?;
    assert_eq!(loaded.volume, 75);

    Ok(())
}
