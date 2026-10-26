use crate::{
    AppState,
    error::{Error, Result},
};
use axum::http::StatusCode;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};

fn input_root() -> PathBuf {
    std::env::var("AUDEXIS_VARIANT_INPUT_DIR")
        .unwrap_or_else(|_| "./data/variant-input".into())
        .into()
}
pub async fn retain_input(_state: &AppState, hash: &str, path: &Path) {
    let root = input_root();
    let result = async {
        tokio::fs::create_dir_all(&root).await?;
        let temp = root.join(format!("{}.part", uuid::Uuid::new_v4()));
        tokio::fs::copy(path, &temp).await?;
        tokio::fs::rename(&temp, root.join(hash)).await
    }
    .await;
    if let Err(error) = result {
        tracing::warn!(%error, "Could not retain input for compressed streaming");
    }
    clean_inputs().await;
}
async fn clean_inputs() {
    let Ok(mut directory) = tokio::fs::read_dir(input_root()).await else {
        return;
    };
    let mut entries = Vec::new();
    while let Ok(Some(entry)) = directory.next_entry().await {
        if let Ok(meta) = entry.metadata().await {
            if meta.is_file() {
                entries.push((
                    meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                    meta.len(),
                    entry.path(),
                ));
            }
        }
    }
    entries.sort_by_key(|entry| entry.0);
    let mut bytes: u64 = entries.iter().map(|entry| entry.1).sum();
    let max = std::env::var("AUDEXIS_VARIANT_INPUT_CACHE_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5_000_000_000u64);
    for (modified, size, path) in entries {
        if bytes > max || modified.elapsed().unwrap_or_default() > Duration::from_secs(86400) {
            if tokio::fs::remove_file(path).await.is_ok() {
                bytes = bytes.saturating_sub(size);
            }
        }
    }
}
pub fn quality(value: &str) -> bool {
    matches!(value, "original" | "aac_128" | "aac_256")
}
pub async fn run(state: Arc<AppState>) {
    if state.ffmpeg.is_none() {
        return;
    }
    let mut tick = tokio::time::interval(Duration::from_secs(10));
    loop {
        tick.tick().await;
        if crate::storage::cooldown_error().is_some() {
            continue;
        }
        let row = sqlx::query("UPDATE cloud_media_variants SET state='processing',lease_at=extract(epoch FROM now())::bigint,attempts=attempts+1 WHERE (object_hash,quality)=(SELECT object_hash,quality FROM cloud_media_variants WHERE (state='queued' AND attempts<3 AND retry_at<=extract(epoch FROM now())::bigint) OR (state='processing' AND lease_at<extract(epoch FROM now())::bigint-600) ORDER BY retry_at FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING object_hash,quality,attempts")
            .fetch_optional(&state.db).await;
        match row {
            Ok(Some(row)) => {
                let hash: String = row.get("object_hash");
                let quality: String = row.get("quality");
                let attempts: i32 = row.get("attempts");
                let encoding = encode(&state, &hash, &quality, attempts);
                tokio::pin!(encoding);
                let mut heartbeat = tokio::time::interval(Duration::from_secs(30));
                heartbeat.tick().await;
                let result = loop {
                    tokio::select! {
                        result = &mut encoding => break result,
                        _ = heartbeat.tick() => {
                            let _ = sqlx::query("UPDATE cloud_media_variants SET lease_at=extract(epoch FROM now())::bigint WHERE object_hash=$1 AND quality=$2 AND state='processing' AND attempts=$3").bind(&hash).bind(&quality).bind(attempts).execute(&state.db).await;
                        }
                    }
                };
                if let Err(error) = result {
                    let cap = crate::storage::cooldown_error().is_some();
                    tracing::warn!(%hash,%quality,reason=%error.1,"Compressed streaming copy failed");
                    let wait = if cap {
                        crate::storage::cooldown_seconds() as i64
                    } else {
                        60 * attempts as i64
                    };
                    let _ = sqlx::query("UPDATE cloud_media_variants SET state=$3,retry_at=extract(epoch FROM now())::bigint+$4,attempts=attempts-$5 WHERE object_hash=$1 AND quality=$2 AND state='processing' AND attempts=$6")
                        .bind(&hash).bind(&quality).bind(if attempts < 3 || cap { "queued" } else { "failed" }).bind(wait).bind(if cap {1} else {0}).bind(attempts).execute(&state.db).await;
                }
            }
            Ok(None) => {}
            Err(error) => tracing::warn!(%error,"Could not claim compressed streaming job"),
        }
        clean_inputs().await;
    }
}
struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
async fn digest(path: PathBuf) -> Result<(i64, String)> {
    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let mut file = std::fs::File::open(path)?;
        let mut digest = Sha256::new();
        let mut size = 0;
        let mut bytes = [0; 65536];
        loop {
            let n = file.read(&mut bytes)?;
            if n == 0 {
                break;
            }
            digest.update(&bytes[..n]);
            size += n as i64;
        }
        Ok((size, format!("{:x}", digest.finalize())))
    })
    .await
    .map_err(|_| Error::bad("Could not verify compressed copy"))?
}
async fn encode(state: &AppState, hash: &str, quality: &str, attempt: i32) -> Result<()> {
    let (key, size): (String, i64) =
        sqlx::query_as("SELECT object_key,size FROM cloud_objects WHERE sha256=$1")
            .bind(hash)
            .fetch_one(&state.db)
            .await?;
    let input =
        Temporary(std::env::temp_dir().join(format!("audexis-encode-{}", uuid::Uuid::new_v4())));
    let local = input_root().join(hash);
    if tokio::fs::copy(local, &input.0).await.is_err() {
        state
            .storage
            .download(&state.client, &key, &input.0, size)
            .await?;
    }
    if digest(input.0.clone()).await? != (size, hash.to_owned()) {
        return Err(Error::bad("Original audio failed verification"));
    }
    let output = Temporary(input.0.with_extension("m4a"));
    transcode(
        state.ffmpeg.as_ref().unwrap(),
        &input.0,
        &output.0,
        quality,
        state.max_file,
    )
    .await?;
    let (bytes, sha) = digest(output.0.clone()).await?;
    if bytes <= 0 || bytes >= state.max_file {
        return Err(Error::bad("Invalid compressed audio output"));
    }
    if bytes >= size {
        sqlx::query("UPDATE cloud_media_variants SET state='unnecessary' WHERE object_hash=$1 AND quality=$2 AND attempts=$3 AND state='processing'")
            .bind(hash).bind(quality).bind(attempt).execute(&state.db).await?;
        return Ok(());
    }
    let object_key = format!("variants/{}", uuid::Uuid::new_v4());
    sqlx::query("INSERT INTO cloud_object_writes(object_key) VALUES ($1)")
        .bind(&object_key)
        .execute(&state.db)
        .await?;
    let mut tx = state.db.begin().await?;
    let lock = i64::from_be_bytes(Sha256::digest(hash.as_bytes())[..8].try_into().unwrap());
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(lock)
        .execute(&mut *tx)
        .await?;
    let current: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cloud_media_variants WHERE object_hash=$1 AND quality=$2 AND state='processing' AND attempts=$3)")
        .bind(hash).bind(quality).bind(attempt).fetch_one(&mut *tx).await?;
    if !current {
        tx.rollback().await?;
        sqlx::query("DELETE FROM cloud_object_writes WHERE object_key=$1")
            .bind(&object_key)
            .execute(&state.db)
            .await?;
        return Ok(());
    }
    state
        .storage
        .put_file(&state.client, &object_key, &output.0, bytes)
        .await?;
    let published = sqlx::query("UPDATE cloud_media_variants SET state='ready',object_key=$3,size=$4,sha256=$5 WHERE object_hash=$1 AND quality=$2 AND state='processing' AND attempts=$6")
        .bind(hash).bind(quality).bind(&object_key).bind(bytes).bind(sha).bind(attempt).execute(&mut *tx).await?.rows_affected();
    tx.commit().await?;
    if published == 0 {
        return Ok(());
    }
    sqlx::query("DELETE FROM cloud_object_writes WHERE object_key=$1")
        .bind(&object_key)
        .execute(&state.db)
        .await?;
    Ok(())
}

async fn transcode(
    executable: &str,
    input: &Path,
    output: &Path,
    quality: &str,
    max_file: i64,
) -> Result<()> {
    let bitrate = if quality == "aac_128" { "128k" } else { "256k" };
    let process = tokio::process::Command::new(executable)
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-protocol_whitelist",
            "file,pipe",
        ])
        .arg("-i")
        .arg(input)
        .args([
            "-map",
            "0:a:0",
            "-vn",
            "-map_metadata",
            "-1",
            "-c:a",
            "aac",
            "-b:a",
            bitrate,
            "-ac",
            "2",
            "-threads",
            "1",
            "-fs",
        ])
        .arg(max_file.to_string())
        .args(["-movflags", "+faststart", "-f", "mp4"])
        .arg(output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let status = tokio::time::timeout(Duration::from_secs(300), process.wait_with_output())
        .await
        .map_err(|_| {
            Error(
                StatusCode::GATEWAY_TIMEOUT,
                "Audio compression timed out".into(),
            )
        })??;
    if !status.status.success() {
        return Err(Error::bad(
            "Audio compression failed; check the server encoder installation",
        ));
    }
    Ok(())
}
