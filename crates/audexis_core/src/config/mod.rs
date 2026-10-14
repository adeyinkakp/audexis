use std::path::PathBuf;
pub mod table;
pub use table::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserConfig {
    pub songs_table: SongsTableConfig,
    pub custom_field_definitions: Vec<CustomFieldDefinition>,
    pub theme: ThemePreference,
    pub density: Density,
    pub reduce_motion: bool,
    pub needs_onboarding: bool,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            songs_table: SongsTableConfig::default(),
            custom_field_definitions: Vec::new(),
            theme: ThemePreference::Light,
            density: Density::Default,
            reduce_motion: false,
            needs_onboarding: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    #[default]
    Default,
    Compact,
    Comfort,
}

impl UserConfig {
    pub fn load_config(path: &PathBuf) -> Self {
        if let Ok(contents) = std::fs::read_to_string(path) {
            match serde_json::from_str(&contents) {
                Ok(cfg) => cfg,
                Err(_) => {
                    let default = UserConfig::default();
                    let _ = UserConfig::save_config(path, &default);
                    default
                }
            }
        } else {
            let default = UserConfig::default();
            let _ = UserConfig::save_config(path, &default);
            default
        }
    }
    pub fn save_config(path: &PathBuf, config: &UserConfig) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(config)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| {
            use std::io::Write;
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(json.as_bytes())?;
            file.sync_all()?;
            std::fs::rename(&temporary, path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }
}
