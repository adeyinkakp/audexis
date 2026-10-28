//! SQLite persistence behind the existing Swift snapshot contract.
use super::{Download, State, library::CoreLibrary, safe_id};
use audexis_core::cloud_sync::{
    apply_to_connection,
    protocol::{Change, ChangeValue},
};
use sqlx::Row;
use std::path::Path;

const CONNECTION: &str = "mobile";

impl CoreLibrary {
    pub(super) fn initialize(&self) -> Result<(), String> {
        self.runtime.block_on(async {

            sqlx::raw_sql(include_str!("mobile_schema.sql")).execute(&self.database.pool).await?;
            sqlx::query("INSERT INTO connections(id,server_url,server_user_id) VALUES (?, 'mobile://library', 'pending') ON CONFLICT(id) DO NOTHING")
                .bind(CONNECTION).execute(&self.database.pool).await?;
            self.database.set_cloud_library_connection(Some(CONNECTION)).await
                .map_err(|e| e.to_string())?;
            Ok::<_, Box<dyn std::error::Error>>(())
        }).map_err(|e| e.to_string())
    }

    pub(super) fn open_state(&self) -> Result<State, String> {
        let initialized: bool = self
            .runtime
            .block_on(
                sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM mobile_library_state WHERE slot=1)",
                )
                .fetch_one(&self.database.pool),
            )
            .map_err(|e| e.to_string())?;
        if !initialized {
            self.save(&State::default(), &State::default())?;
        }
        self.load()
    }

    pub(super) fn load(&self) -> Result<State, String> {
        self.runtime.block_on(async {
            self.database.refresh_cloud_library(CONNECTION).await.map_err(|e| e.to_string())?;
            let mut tx = self.database.pool.begin().await?;
            let row = sqlx::query("SELECT user_id,current_index,generation FROM mobile_library_state WHERE slot=1")
                .fetch_one(&mut *tx).await?;
            let mut state = State {
                user_id: row.get("user_id"),
                current: row.get::<Option<i64>, _>("current_index").map(|i| i as usize),
                generation: row.get("generation"),
                ..State::default()
            };
            state.cursor = sqlx::query_scalar("SELECT cursor FROM cloud_sync_cursors WHERE connection_id=?")
                .bind(CONNECTION).fetch_optional(&mut *tx).await?.unwrap_or(0);
            let docs: Vec<String> = sqlx::query_scalar("SELECT document FROM cloud_tracks WHERE connection_id=? AND json_extract(document,'$.deleted')=0")
                .bind(CONNECTION).fetch_all(&mut *tx).await?;
            for doc in docs {
                let track: audexis_protocol::Track = serde_json::from_str(&doc).map_err(|e| e.to_string())?;
                state.tracks.insert(track.id.clone(), track);
            }
            let docs: Vec<String> = sqlx::query_scalar("SELECT document FROM cloud_playlists WHERE connection_id=? AND json_extract(document,'$.deleted')=0")
                .bind(CONNECTION).fetch_all(&mut *tx).await?;
            for doc in docs {
                let playlist: audexis_protocol::Playlist = serde_json::from_str(&doc).map_err(|e| e.to_string())?;
                state.playlists.insert(playlist.id.clone(), playlist);
            }
            state.queue = sqlx::query_scalar("SELECT track_id FROM mobile_queue ORDER BY position")
                .fetch_all(&mut *tx).await?;
            for row in sqlx::query("SELECT track_id,file,source_hash,sha256,size FROM mobile_downloads")
                .fetch_all(&mut *tx).await? {
                state.downloads.insert(row.get("track_id"), Download {
                    file: row.get("file"), source_hash: row.get("source_hash"),
                    sha256: row.get("sha256"), size: row.get("size"),
                });
            }
            tx.commit().await?;
            Ok::<_, Box<dyn std::error::Error>>(state)
        }).map_err(|e| e.to_string())
    }

    pub(super) fn save(&self, state: &State, previous: &State) -> Result<(), String> {
        validate(state)?;
        self.runtime.block_on(async {
            let mut tx = self.database.pool.begin_with("BEGIN IMMEDIATE").await?;
            let generation: i64 = sqlx::query_scalar("SELECT COALESCE((SELECT generation FROM mobile_library_state WHERE slot=1),0)")
                .fetch_one(&mut *tx).await?;
            if generation != previous.generation {
                return Err("The library changed. Refresh and try again.".into());
            }
            let mut changes = Vec::new();
            for track in state.tracks.values() {
                if previous.tracks.get(&track.id).map(serde_json::to_string).transpose()? != Some(serde_json::to_string(track)?) {
                    changes.push(Change { cursor: 0, value: ChangeValue::Track(track.clone()) });
                }
            }
            for track in previous.tracks.values().filter(|t| !state.tracks.contains_key(&t.id)) {
                let mut deleted = track.clone(); deleted.deleted = true;
                changes.push(Change { cursor: 0, value: ChangeValue::Track(deleted) });
            }
            for playlist in state.playlists.values() {
                if previous.playlists.get(&playlist.id).map(serde_json::to_string).transpose()? != Some(serde_json::to_string(playlist)?) {
                    changes.push(Change { cursor: 0, value: ChangeValue::Playlist(playlist.clone()) });
                }
            }
            for playlist in previous.playlists.values().filter(|p| !state.playlists.contains_key(&p.id)) {
                let mut deleted = playlist.clone(); deleted.deleted = true;
                changes.push(Change { cursor: 0, value: ChangeValue::Playlist(deleted) });
            }
            apply_to_connection(&mut tx, CONNECTION, &changes, Some(state.cursor)).await.map_err(|e| e.to_string())?;
            sqlx::query("INSERT INTO mobile_library_state(slot,user_id,current_index,generation) VALUES (1,?,?,?) ON CONFLICT(slot) DO UPDATE SET user_id=excluded.user_id,current_index=excluded.current_index,generation=excluded.generation")
                .bind(&state.user_id).bind(state.current.map(|i| i as i64)).bind(generation+1).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM mobile_queue").execute(&mut *tx).await?;
            for (index, id) in state.queue.iter().enumerate() {
                sqlx::query("INSERT INTO mobile_queue(position,track_id) VALUES (?,?)")
                    .bind(index as i64).bind(id).execute(&mut *tx).await?;
            }
            sqlx::query("DELETE FROM mobile_downloads").execute(&mut *tx).await?;
            for (id, d) in &state.downloads {
                sqlx::query("INSERT INTO mobile_downloads(track_id,file,source_hash,sha256,size) VALUES (?,?,?,?,?)")
                    .bind(id).bind(&d.file).bind(&d.source_hash).bind(&d.sha256).bind(d.size).execute(&mut *tx).await?;
            }
            tx.commit().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        }).map_err(|e| e.to_string())
    }
}

fn validate(state: &State) -> Result<(), String> {
    if state.cursor < 0
        || state.current.is_some_and(|i| i >= state.queue.len())
        || state.queue.iter().any(|id| !state.tracks.contains_key(id))
    {
        return Err("Invalid saved library queue or cursor".into());
    }
    for (id, track) in &state.tracks {
        if id != &track.id || !safe_id(id) {
            return Err("Invalid saved track ID".into());
        }
    }
    for (id, playlist) in &state.playlists {
        if id != &playlist.id || !safe_id(id) {
            return Err("Invalid saved playlist ID".into());
        }
    }
    for (id, d) in &state.downloads {
        let mut components = Path::new(&d.file).components();
        if !state.tracks.contains_key(id)
            || d.size <= 0
            || !matches!(components.next(), Some(std::path::Component::Normal(_)))
            || components.next().is_some()
        {
            return Err("Invalid saved download".into());
        }
    }
    Ok(())
}
