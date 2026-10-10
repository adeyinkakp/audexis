use crate::{
    audio_player::equalizer::{self, Settings},
    AppState,
};
use tauri_plugin_store::StoreExt;

const STORE: &str = "equalizer.json";

pub fn load(app: &tauri::AppHandle) -> Settings {
    let result = (|| -> Result<Settings, String> {
        let store = app.store(STORE).map_err(|e| e.to_string())?;
        let settings: Settings = match store.get("settings") {
            Some(value) => serde_json::from_value(value).map_err(|e| e.to_string())?,
            None => Settings::default(),
        };
        settings.validate()?;
        Ok(settings)
    })();
    result.unwrap_or_else(|error| {
        tauri_plugin_log::log::warn!("Could not restore EQ; using bypass: {error}");
        Settings::default()
    })
}

#[derive(serde::Serialize)]
pub struct Config {
    settings: Settings,
    presets: Vec<equalizer::PresetInfo>,
    frequencies: [f64; 10],
}

#[tauri::command]
pub fn get_equalizer(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player unavailable")?;
    let settings = *player
        .equalizer
        .lock()
        .map_err(|_| "Equalizer unavailable")?;
    Ok(Config {
        settings,
        presets: equalizer::presets(),
        frequencies: equalizer::FREQUENCIES,
    })
}

#[tauri::command]
pub fn set_equalizer(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    settings.validate()?;
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player unavailable")?;
    let store = app.store(STORE).map_err(|e| e.to_string())?;
    let previous = store.get("settings");
    store.set(
        "settings",
        serde_json::to_value(settings).map_err(|e| e.to_string())?,
    );
    if let Err(error) = store.save() {
        match previous {
            Some(value) => store.set("settings", value),
            None => {
                store.delete("settings");
            }
        }
        return Err(format!("Could not save equalizer: {error}"));
    }
    *player
        .equalizer
        .lock()
        .map_err(|_| "Equalizer unavailable")? = settings;
    Ok(())
}
