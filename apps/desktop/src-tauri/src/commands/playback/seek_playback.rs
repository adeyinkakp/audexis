use tauri::command;

use crate::AppState;

#[command]
pub fn seek_playback(
    state: tauri::State<'_, AppState>,
    seconds: Option<u64>,
    milliseconds: Option<u64>,
) -> Result<(), String> {
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    if let Some(milliseconds) = milliseconds {
        player.seek_milliseconds(milliseconds);
    } else if let Some(seconds) = seconds {
        player.seek(seconds);
    } else {
        return Err("Provide a playback position".into());
    }
    Ok(())
}
