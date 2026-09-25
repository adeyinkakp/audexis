use tauri::{command, AppHandle};

use crate::AppState;

use super::shared::{
    load_now_playing, previous_queue_path, publish_now_playing, restart_current_track,
};

#[command]
pub async fn previous_song(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let path = previous_queue_path(&state)?;
    restart_current_track(&app, &state, &path).await?;
    let info = load_now_playing(&state, &path).await?;
    publish_now_playing(&app, &state, info)
}
