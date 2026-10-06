use crate::audio_player::lyrics_preview::{Action, Preview, Status};
use crate::AppState;
use std::sync::Mutex;
use tauri::Manager;

#[derive(Default)]
pub struct LyricsPreviewState(Mutex<Option<Preview>>);

#[tauri::command]
pub async fn lyrics_preview(
    app: tauri::AppHandle,
    action: Action,
    session_id: String,
    file_id: Option<i64>,
    position_ms: Option<u64>,
) -> Result<Status, String> {
    let path = if matches!(action, Action::Open) {
        let state = app.state::<AppState>();
        Some(
            sqlx::query_scalar::<_, String>("SELECT path FROM files WHERE id = ?")
                .bind(file_id.ok_or("Select a track to preview")?)
                .fetch_optional(&state.db.pool)
                .await
                .map_err(|e| e.to_string())?
                .ok_or("Track no longer exists")?,
        )
    } else {
        None
    };
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<LyricsPreviewState>();
        let mut current = state.0.lock().map_err(|_| "Preview unavailable")?;
        if let Some(path) = path {
            *current = None;
            let equalizer = app
                .state::<AppState>()
                .audio_player
                .lock()
                .map_err(|_| "Audio player unavailable")?
                .equalizer
                .clone();
            let (preview, status) = Preview::open(session_id, path, equalizer)?;
            *current = Some(preview);
            return Ok(status);
        }
        let Some(preview) = current.as_ref().filter(|p| p.session_id == session_id) else {
            return if matches!(action, Action::Close) {
                Ok(Status::default())
            } else {
                Err("Preview session has closed".into())
            };
        };
        if matches!(action, Action::Close) {
            *current = None;
            return Ok(Status::default());
        }
        if matches!(action, Action::Play) {
            app.state::<AppState>()
                .audio_player
                .lock()
                .map_err(|_| "Audio player unavailable")?
                .pause();
        }
        preview.request(action, position_ms.unwrap_or(0))
    })
    .await
    .map_err(|e| e.to_string())?
}
