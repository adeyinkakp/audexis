mod worker;
pub use worker::*;

mod operations;
use operations::*;

mod pull;
use pull::*;

mod media;

mod mutations;

mod recovery;

use super::{
    client::{CloudClient, Result, SyncError},
    protocol::*,
    store::{self, Job, Pending},
};
use crate::{database::Database, library_files::LibraryFiles};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

type ArtworkLocks =
    std::collections::HashMap<(String, i64), std::sync::Weak<tokio::sync::Mutex<()>>>;
static ARTWORK_LOCKS: std::sync::LazyLock<std::sync::Mutex<ArtworkLocks>> =
    std::sync::LazyLock::new(Default::default);

#[derive(Clone)]
pub struct CloudSyncContext {
    pub db: Database,
    pub connection_id: String,
    pub server_user_id: String,
    pub server_url: String,
    pub api_key: String,
    pub cache_dir: PathBuf,
    pub files: Arc<LibraryFiles>,
}
impl CloudSyncContext {
    pub async fn refresh_library(&self) -> Result<()> {
        self.reconcile_downloads().await?;
        if super::library::refresh(&self.db, &self.connection_id).await? {
            self.files.publish_changed(Vec::new());
        }
        Ok(())
    }
    pub async fn ensure_library_artwork(&self, file_id: i64, path: &str) -> Result<()> {
        static FETCHES: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
        let lock = {
            let mut locks = ARTWORK_LOCKS
                .lock()
                .map_err(|_| SyncError::local("Artwork loader unavailable"))?;
            locks.retain(|_, lock| lock.strong_count() > 0);
            let key = (self.connection_id.clone(), file_id);
            if let Some(lock) = locks.get(&key).and_then(std::sync::Weak::upgrade) {
                lock
            } else {
                let lock = Arc::new(tokio::sync::Mutex::new(()));
                locks.insert(key, Arc::downgrade(&lock));
                lock
            }
        };
        let _guard = lock.lock().await;
        let _permit = FETCHES
            .acquire()
            .await
            .map_err(|_| SyncError::local("Artwork loader unavailable"))?;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM files f JOIN cloud_library_session s ON s.connection_id=f.cloud_connection_id WHERE f.id=? AND f.path=? AND f.cloud_connection_id=? AND f.missing_since IS NULL)")
            .bind(file_id).bind(path).bind(&self.connection_id).fetch_one(&self.db.pool).await?;
        if !exists {
            return Err(SyncError::local(
                "This song is no longer in the active cloud library",
            ));
        }
        let stored: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM metadata_pictures WHERE file_id=?)")
                .bind(file_id)
                .fetch_one(&self.db.pool)
                .await?;
        if stored {
            return Ok(());
        }
        let track = self.library_track(path).await?;
        if !track.artwork {
            return Ok(());
        }
        let cache = self.artwork_path(&track);
        if !tokio::fs::try_exists(&cache).await? {
            let client = self.client()?;
            let media = client.media(&track.id, "artwork").await?;
            client
                .download(&media.url, &cache, None, 16 * 1024 * 1024)
                .await?;
        }
        let size = tokio::fs::metadata(&cache).await?.len();
        if size == 0 || size > 16 * 1024 * 1024 {
            let _ = tokio::fs::remove_file(&cache).await;
            return Err(SyncError::local("Cloud artwork has an invalid size"));
        }
        let bytes = tokio::fs::read(&cache).await?;
        let format = image::guess_format(&bytes)
            .map_err(|_| SyncError::local("Cloud artwork is not a supported image"))?;
        sqlx::query("INSERT INTO metadata_pictures(file_id,data,mime_type,picture_type) SELECT f.id,?,?,3 FROM files f JOIN cloud_library_session s ON s.connection_id=f.cloud_connection_id WHERE f.id=? AND f.path=? AND f.cloud_connection_id=? AND f.missing_since IS NULL AND NOT EXISTS(SELECT 1 FROM metadata_pictures p WHERE p.file_id=f.id)")
            .bind(bytes).bind(format.to_mime_type()).bind(file_id).bind(path).bind(&self.connection_id).execute(&self.db.pool).await?;
        Ok(())
    }
    pub async fn library_track(&self, path: &str) -> Result<Track> {
        let prefix = format!("cloud-library://{}/", self.connection_id);
        let id = path
            .strip_prefix(&prefix)
            .ok_or_else(|| SyncError::local("This song belongs to another cloud account"))?;
        let document: String = sqlx::query_scalar("SELECT document FROM cloud_tracks WHERE connection_id=? AND id=? AND json_extract(document,'$.deleted')=0")
            .bind(&self.connection_id).bind(id).fetch_optional(&self.db.pool).await?
            .ok_or_else(|| SyncError::local("This song is no longer in your cloud library"))?;
        Ok(serde_json::from_str(&document)?)
    }

    pub fn client(&self) -> Result<CloudClient> {
        CloudClient::new(&self.server_url, &self.api_key)
    }
    pub fn cache_path(&self, track: &Track) -> PathBuf {
        let name = format!("{:x}", Sha256::digest(track.id.as_bytes()));
        let extension = Path::new(&track.file_name)
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| e.len() < 10 && e.bytes().all(|b| b.is_ascii_alphanumeric()))
            .unwrap_or("audio");
        self.cache_dir
            .join(&self.connection_id)
            .join("audio")
            .join(format!("{name}.{extension}"))
    }
    pub fn artwork_path(&self, track: &Track) -> PathBuf {
        self.cache_dir
            .join(&self.connection_id)
            .join("artwork")
            .join(format!("{:x}.png", Sha256::digest(track.id.as_bytes())))
    }
    pub async fn tracks(&self, search: &str, offset: i64) -> Result<Vec<Track>> {
        let docs:Vec<String>=sqlx::query_scalar("SELECT document FROM cloud_tracks WHERE connection_id=? AND json_extract(document,'$.deleted')=0 AND (json_extract(document,'$.file_name') LIKE ? OR json_extract(document,'$.metadata.tags') LIKE ?) ORDER BY COALESCE(json_extract(document,'$.metadata.tags.title[0]'),json_extract(document,'$.file_name')) COLLATE NOCASE,id LIMIT 100 OFFSET ?").bind(&self.connection_id).bind(format!("%{search}%")).bind(format!("%{search}%")).bind(offset.max(0)).fetch_all(&self.db.pool).await?;
        docs.into_iter()
            .map(|d| serde_json::from_str(&d).map_err(Into::into))
            .collect()
    }
    pub async fn playlists(&self) -> Result<Vec<Playlist>> {
        let docs:Vec<String>=sqlx::query_scalar("SELECT document FROM cloud_playlists WHERE connection_id=? AND json_extract(document,'$.deleted')=0 ORDER BY json_extract(document,'$.name') COLLATE NOCASE").bind(&self.connection_id).fetch_all(&self.db.pool).await?;
        docs.into_iter()
            .map(|d| serde_json::from_str(&d).map_err(Into::into))
            .collect()
    }
}
