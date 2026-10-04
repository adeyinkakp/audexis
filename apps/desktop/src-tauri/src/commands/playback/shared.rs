use crate::{
    audio_player::{PlaybackModes, RepeatMode},
    AppState,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tauri::{AppHandle, Emitter};

use crate::audio_player::queue_track::UnresolvedTrack;

#[derive(Debug, Clone, Serialize)]
pub struct NowPlayingInfo {
    pub path: String,
    pub title: String,
    pub album: String,
    pub artist: String,
    pub genre: String,
    pub artwork_url: Option<String>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueInfo {
    pub paths: Vec<String>,
    pub file_ids: Vec<i64>,
    pub occurrences: Vec<Option<i64>>,
    pub current_index: i32,
    pub playlist_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayBackInfo {
    pub curr: UnresolvedTrack,
    pub queue: Vec<UnresolvedTrack>,
    #[serde(alias = "currentIndex")]
    pub current_index: Option<usize>,
    #[serde(alias = "playlistId")]
    pub playlist_id: Option<i64>,
}

pub fn get_queue_info(state: &tauri::State<'_, AppState>) -> Result<QueueInfo, String> {
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    let queue = player
        .queue
        .lock()
        .map_err(|_| "Audio queue is unavailable".to_string())?;
    Ok(QueueInfo {
        paths: queue.paths(),
        file_ids: queue.file_ids(),
        occurrences: queue.playlist_ordinals(),
        current_index: queue.index,
        playlist_id: queue.playlist_id,
    })
}

pub fn emit_queue_changed(
    app: &AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<(), String> {
    let queue = get_queue_info(state)?;
    app.emit("queue-changed", queue)
        .map_err(|error| error.to_string())
}

pub async fn reconcile_queue_with_library(
    app: &AppHandle,
    state: &tauri::State<'_, AppState>,
    path_updates: HashMap<i64, String>,
    missing_ids: HashSet<i64>,
) -> Result<(), String> {
    if path_updates.is_empty() && missing_ids.is_empty() {
        return Ok(());
    }
    let outcome = {
        let player = state
            .audio_player
            .lock()
            .map_err(|_| "Audio player is unavailable".to_string())?;
        let outcome = player
            .queue
            .lock()
            .map_err(|_| "Audio queue is unavailable".to_string())?
            .reconcile_library_files(&path_updates, &missing_ids);
        if outcome.current_removed {
            if outcome.current_path.is_some() {
                player.restart();
            } else {
                player.stop();
            }
        }
        outcome
    };

    emit_queue_changed(app, state)?;
    if outcome.current_removed {
        if let Some(path) = outcome.current_path {
            let info = load_now_playing(state, &path).await?;
            publish_now_playing(app, state, info)?;
        } else {
            *state
                .now_playing
                .lock()
                .map_err(|_| "Now playing state is unavailable".to_string())? = None;
            app.emit("now-playing-changed", Option::<NowPlayingInfo>::None)
                .map_err(|error| error.to_string())?;
            app.emit("playback-queue-done", ())
                .map_err(|error| error.to_string())?;
        }
    } else if outcome.current_path_changed {
        let updated = {
            let mut now_playing = state
                .now_playing
                .lock()
                .map_err(|_| "Now playing state is unavailable".to_string())?;
            if let (Some(info), Some(path)) = (now_playing.as_mut(), outcome.current_path) {
                info.path = path;
            }
            now_playing.clone()
        };
        app.emit("now-playing-changed", updated)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn get_playback_modes_info(
    state: &tauri::State<'_, AppState>,
) -> Result<PlaybackModes, String> {
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    let queue = player
        .queue
        .lock()
        .map_err(|_| "Audio queue is unavailable".to_string())?;
    Ok(queue.playback_modes())
}

pub fn emit_playback_modes_changed(
    app: &AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<(), String> {
    let modes = get_playback_modes_info(state)?;
    app.emit("playback-modes-changed", modes)
        .map_err(|error| error.to_string())
}

pub fn finish_playback_and_clear_queue(
    app: &AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<(), String> {
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    player.stop();
    player
        .queue
        .lock()
        .map_err(|_| "Audio queue is unavailable".to_string())?
        .clear();
    drop(player);

    *state
        .now_playing
        .lock()
        .map_err(|_| "Now playing state is unavailable".to_string())? = None;

    app.emit("now-playing-changed", Option::<NowPlayingInfo>::None)
        .map_err(|error| error.to_string())?;
    app.emit(
        "queue-changed",
        QueueInfo {
            paths: Vec::new(),
            file_ids: Vec::new(),
            occurrences: Vec::new(),
            current_index: 0,
            playlist_id: None,
        },
    )
    .map_err(|error| error.to_string())?;
    app.emit(
        "playback-modes-changed",
        PlaybackModes {
            repeat_mode: RepeatMode::Off,
            shuffled: false,
        },
    )
    .map_err(|error| error.to_string())?;
    app.emit("playback-queue-done", ())
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn publish_now_playing(
    app: &AppHandle,
    state: &tauri::State<'_, AppState>,
    info: NowPlayingInfo,
) -> Result<(), String> {
    *state
        .now_playing
        .lock()
        .map_err(|_| "Now playing state is unavailable".to_string())? = Some(info.clone());
    app.emit("now-playing-changed", info)
        .map_err(|error| error.to_string())
}

pub fn next_queue_path(state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    let mut queue = player
        .queue
        .lock()
        .map_err(|_| "Audio queue is unavailable".to_string())?;
    queue.next().ok_or_else(|| "End of queue".to_string())
}

pub fn previous_queue_path(state: &tauri::State<'_, AppState>) -> Result<String, String> {
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    let mut queue = player
        .queue
        .lock()
        .map_err(|_| "Audio queue is unavailable".to_string())?;
    queue.previous().ok_or_else(|| "Start of queue".to_string())
}

pub async fn restart_current_track(
    app: &AppHandle,
    state: &tauri::State<'_, AppState>,
    _path: &str,
) -> Result<(), String> {
    let player = state
        .audio_player
        .lock()
        .map_err(|_| "Audio player is unavailable".to_string())?;
    player.restart();
    drop(player);
    emit_queue_changed(app, state)
}

pub async fn load_now_playing(
    state: &tauri::State<'_, AppState>,
    path: &str,
) -> Result<NowPlayingInfo, String> {
    let file_id: i64 = sqlx::query_scalar("SELECT id FROM files WHERE path = ?1")
        .bind(path)
        .fetch_optional(&state.db.pool)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Song is no longer in the library".to_string())?;
    let duration_ms =
        sqlx::query_scalar::<_, Option<i64>>("SELECT duration_ms FROM files WHERE id = ?1")
            .bind(file_id)
            .fetch_one(&state.db.pool)
            .await
            .map_err(|error| error.to_string())?;

    let value = |key: &'static str| async move {
        sqlx::query_scalar::<_, String>(
            "SELECT value FROM metadata_texts
             WHERE file_id = ?1 AND key = ?2
             ORDER BY ord LIMIT 1",
        )
        .bind(file_id)
        .bind(key)
        .fetch_optional(&state.db.pool)
        .await
        .map_err(|error| error.to_string())
    };

    let title = value("title").await?.unwrap_or_else(|| {
        std::path::Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Unknown song")
            .to_string()
    });
    let album = value("album").await?.unwrap_or_default();
    let artist = value("artist").await?.unwrap_or_default();
    let genre = value("genre").await?.unwrap_or_default();
    let artwork = sqlx::query_as::<_, (Vec<u8>, String)>(
        "SELECT data, mime_type FROM metadata_pictures
         WHERE file_id = ?1 ORDER BY id LIMIT 1",
    )
    .bind(file_id)
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|error| error.to_string())?;
    let artwork_url = artwork.map(|(data, mime)| {
        format!(
            "data:{};base64,{}",
            mime,
            base64::engine::general_purpose::STANDARD.encode(data)
        )
    });

    Ok(NowPlayingInfo {
        path: path.to_string(),
        title,
        album,
        artist,
        genre,
        artwork_url,
        duration_ms,
    })
}
