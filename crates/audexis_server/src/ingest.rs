mod cleanup;
use cleanup::*;

mod worker;
pub use worker::*;

use crate::{
    AppState, db,
    error::{Error, Result},
    variants,
};
use audexis_core::{
    cloud_sync::protocol::*,
    tag_manager::{
        tag_backend::{DefaultBackend, TagBackend},
        utils::TagValue,
    },
};
use axum::http::StatusCode;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::{
    io::{Cursor, Read},
    path::PathBuf,
    sync::Arc,
};

struct TempFile(PathBuf);
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
pub(crate) async fn artwork_descriptor(state: &AppState, key: &str) -> Result<(i64, String)> {
    static SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
    let _slot = SLOTS
        .acquire()
        .await
        .map_err(|_| Error::bad("Artwork processing unavailable"))?;
    let temp =
        TempFile(std::env::temp_dir().join(format!("audexis-artwork-{}", uuid::Uuid::new_v4())));
    state
        .storage
        .download(&state.client, key, &temp.0, 20_000_000)
        .await?;
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(&temp.0).await?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut size = 0i64;
    loop {
        let count = file.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
        size += count as i64;
    }
    Ok((size, format!("{:x}", digest.finalize())))
}
pub(crate) async fn process(
    state: &AppState,
    id: &str,
    user: &str,
    stage: &str,
    request: UploadRequest,
) -> Result<()> {
    let ext = std::path::Path::new(&request.file_name)
        .extension()
        .and_then(|e| e.to_str())
        .filter(|s| s.len() < 10 && s.bytes().all(|b| b.is_ascii_alphanumeric()))
        .unwrap_or("audio");
    let temp = TempFile(std::env::temp_dir().join(format!(
        "audexis-ingest-{}.{}",
        uuid::Uuid::new_v4(),
        ext
    )));
    let size = if let Ok(path) = state.stage_path(stage) {
        if tokio::fs::try_exists(&path).await? {
            let size = tokio::fs::metadata(&path).await?.len();
            if size > request.size as u64 {
                return Err(Error::bad("Uploaded file exceeds allowed size"));
            }
            tokio::fs::copy(path, &temp.0).await?;
            size as i64
        } else {
            state
                .storage
                .download(&state.client, stage, &temp.0, request.size)
                .await?
        }
    } else {
        state
            .storage
            .download(&state.client, stage, &temp.0, request.size)
            .await?
    };
    if size != request.size {
        return Err(Error::bad("Uploaded file size does not match"));
    }
    let path = temp.0.clone();
    let expected = request.sha256.clone();
    let inspected = tokio::task::spawn_blocking(move || -> Result<_> {
        let mut input = std::fs::File::open(&path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let n = input.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        let hash = format!("{:x}", hasher.finalize());
        if hash != expected {
            return Err(Error::bad("Uploaded file SHA-256 does not match"));
        }
        let media = symphonia::core::io::MediaSourceStream::new(
            Box::new(std::fs::File::open(&path)?),
            Default::default(),
        );
        let probed = symphonia::default::get_probe()
            .probe(
                &Default::default(),
                media,
                Default::default(),
                Default::default(),
            )
            .map_err(|_| Error::bad("Unsupported or invalid audio file"))?;
        let audio_track = probed
            .default_track(symphonia::core::formats::TrackType::Audio)
            .ok_or_else(|| Error::bad("File has no audio track"))?;
        let duration_ms =
            audio_track
                .time_base
                .zip(audio_track.duration)
                .map(|(base, duration)| {
                    ((duration.get() as u128 * base.numer.get() as u128 * 1000)
                        / base.denom.get() as u128)
                        .min(i64::MAX as u128) as i64
                });
        let metadata = DefaultBackend::new().read(&path).ok();
        let mut tags = Tags::new();
        let mut artwork = None;
        if let Some(metadata) = &metadata {
            for (key, values) in &metadata.tags {
                for value in values {
                    match value {
                        TagValue::Picture { data, .. } => {
                            if artwork.is_none() && data.len() <= 20_000_000 {
                                let mut reader = image::ImageReader::new(Cursor::new(data))
                                    .with_guessed_format()
                                    .map_err(|_| Error::bad("Invalid artwork"))?;
                                let mut limits = image::Limits::default();
                                limits.max_image_width = Some(8192);
                                limits.max_image_height = Some(8192);
                                limits.max_alloc = Some(64_000_000);
                                reader.limits(limits);
                                if let Ok(image) = reader.decode() {
                                    let mut output = Cursor::new(Vec::new());
                                    if image
                                        .thumbnail(512, 512)
                                        .write_to(&mut output, image::ImageFormat::Png)
                                        .is_ok()
                                    {
                                        artwork = Some(output.into_inner())
                                    }
                                }
                            }
                        }
                        _ => {
                            tags.entry(key.to_string())
                                .or_default()
                                .push(value.to_string());
                        }
                    }
                }
            }
        }
        let format = metadata
            .map(|m| m.tag_format.to_string())
            .unwrap_or_default();
        Ok((hash, format, tags, artwork, duration_ms))
    })
    .await
    .map_err(|_| {
        Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not inspect upload".into(),
        )
    })??;
    let (hash, format, _embedded_tags, artwork, duration_ms) = inspected;
    let metadata = request.metadata.clone();
    let fresh_audio = format!("audio/{}", uuid::Uuid::new_v4());
    let fresh_art = artwork
        .as_ref()
        .map(|_| format!("artwork/{}", uuid::Uuid::new_v4()));
    let mut written_keys = vec![fresh_audio.clone()];
    if let Some(key) = &fresh_art {
        written_keys.push(key.clone());
    }
    sqlx::query("INSERT INTO cloud_object_writes(object_key) SELECT unnest($1::text[])")
        .bind(&written_keys)
        .execute(&state.db)
        .await?;
    let mut tx = state.db.begin().await?;
    let lock = i64::from_be_bytes(Sha256::digest(hash.as_bytes())[..8].try_into().unwrap());
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(lock)
        .execute(&mut *tx)
        .await?;
    let exists: Option<String> =
        sqlx::query_scalar("SELECT sha256 FROM cloud_objects WHERE sha256=$1 FOR UPDATE")
            .bind(&hash)
            .fetch_optional(&mut *tx)
            .await?;
    if exists.is_none() {
        let key = fresh_audio;
        state
            .storage
            .put_file(&state.client, &key, &temp.0, size)
            .await?;
        let artwork_size = artwork.as_ref().map(|bytes| bytes.len() as i64);
        let artwork_hash = artwork
            .as_ref()
            .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
        let artwork_key = if let Some(bytes) = artwork {
            let key = fresh_art.expect("artwork key allocated for this thumbnail");
            let image_temp = TempFile(
                std::env::temp_dir().join(format!("audexis-art-{}", uuid::Uuid::new_v4())),
            );
            tokio::fs::write(&image_temp.0, &bytes).await?;
            state
                .storage
                .put_file(&state.client, &key, &image_temp.0, bytes.len() as i64)
                .await?;
            Some(key)
        } else {
            None
        };
        sqlx::query("INSERT INTO cloud_objects(sha256,object_key,size,format,artwork_key,duration_ms,artwork_size,artwork_sha256) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)").bind(&hash).bind(&key).bind(size).bind(&format).bind(&artwork_key).bind(duration_ms).bind(artwork_size).bind(artwork_hash).execute(&mut *tx).await?;
    }
    let quota = db::lock_account(&mut tx, user).await?;
    let existing: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT document FROM cloud_tracks WHERE user_id=$1 AND object_hash=$2 FOR UPDATE",
    )
    .bind(user)
    .bind(&hash)
    .fetch_optional(&mut *tx)
    .await?;
    let mut track: Track = if let Some(value) = existing {
        serde_json::from_value(value)?
    } else {
        Track {
            id: uuid::Uuid::new_v4().to_string(),
            revision: 0,
            sha256: hash.clone(),
            size,
            file_name: request.file_name,
            format,
            duration_ms: None,
            metadata: metadata.clone(),
            artwork: false,
            deleted: true,
        }
    };
    if track.deleted {
        let used:i64=sqlx::query_scalar("SELECT COALESCE(sum(o.size),0)::bigint FROM cloud_tracks t JOIN cloud_objects o ON o.sha256=t.object_hash WHERE t.user_id=$1 AND NOT t.deleted").bind(user).fetch_one(&mut *tx).await?;
        if used.saturating_add(size) > quota {
            return Err(Error(
                StatusCode::INSUFFICIENT_STORAGE,
                "Cloud storage quota exceeded".into(),
            ));
        }
        let (format, duration, has_art): (String, Option<i64>, bool) = sqlx::query_as(
            "SELECT format,duration_ms,artwork_key IS NOT NULL FROM cloud_objects WHERE sha256=$1",
        )
        .bind(&hash)
        .fetch_one(&mut *tx)
        .await?;
        track.revision += 1;
        track.deleted = false;
        track.metadata = metadata;
        track.format = format;
        track.duration_ms = duration;
        track.artwork = has_art;
        sqlx::query("INSERT INTO cloud_tracks(id,user_id,object_hash,document) VALUES ($1,$2,$3,$4) ON CONFLICT(id) DO UPDATE SET document=excluded.document,deleted=FALSE").bind(&track.id).bind(user).bind(&hash).bind(serde_json::to_value(&track)?).execute(&mut *tx).await?;
        db::change(&mut tx, user, &ChangeValue::Track(track.clone())).await?;
    }
    sqlx::query("UPDATE cloud_uploads SET state='ready',track_id=$2,error=NULL WHERE id=$1 AND state='processing'").bind(id).bind(&track.id).execute(&mut *tx).await?;
    sqlx::query("UPDATE cloud_objects SET unreferenced_at=NULL WHERE sha256=$1")
        .bind(&hash)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    if state.ffmpeg.is_some() {
        variants::retain_input(state, &hash, &temp.0).await;
    }
    sqlx::query("DELETE FROM cloud_object_writes WHERE object_key=ANY($1)")
        .bind(written_keys)
        .execute(&state.db)
        .await?;
    Ok(())
}
