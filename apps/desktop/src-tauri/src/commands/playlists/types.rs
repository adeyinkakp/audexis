use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PlaylistSummary {
    pub id: i64,
    pub name: String,
    pub track_count: i64,
    pub artwork_file_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PlaylistTrack {
    pub id: i64,
    pub path: String,
    pub file_name: String,
    pub duration_ms: Option<i64>,
    pub ord: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaylistDetail {
    pub id: i64,
    pub name: String,
    pub tracks: Vec<PlaylistTrack>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePlaylistInput {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct RenamePlaylistInput {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AddPlaylistTrackInput {
    pub playlist_id: i64,
    pub file_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct RemovePlaylistTrackInput {
    pub playlist_id: i64,
    pub ord: i64,
}

#[derive(Debug, Deserialize)]
pub struct ReorderPlaylistTrackInput {
    pub playlist_id: i64,
    pub from_ord: i64,
    pub to_ord: i64,
}
