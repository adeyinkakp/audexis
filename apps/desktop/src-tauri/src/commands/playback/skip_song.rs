use tauri::{command, AppHandle};

use crate::AppState;

use super::shared::{
    finish_playback_and_clear_queue, load_now_playing, next_queue_path, publish_now_playing,
    restart_current_track,
};

#[command]
pub async fn skip_song(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let path = match next_queue_path(&state) {
        Ok(path) => path,
        Err(error) if error == "End of queue" => {
            finish_playback_and_clear_queue(&app, &state)?;
            return Ok(());
        }
        Err(error) => return Err(error),
    };

    restart_current_track(&app, &state, &path).await?;
    let info = load_now_playing(&state, &path).await?;
    publish_now_playing(&app, &state, info)
}
