use crate::{
    AppState,
    error::{Error, Result},
};
use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_util::io::ReaderStream;

pub(super) async fn local_upload(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    request: Request,
) -> Result<StatusCode> {
    static SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
    let _slot = SLOTS.try_acquire().map_err(|_| {
        Error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Upload server is busy. Try again shortly.".into(),
        )
    })?;
    let ticket = state
        .tickets
        .lock()
        .await
        .remove(&token)
        .filter(|t| t.write && t.expires > Instant::now())
        .ok_or_else(|| Error(StatusCode::UNAUTHORIZED, "Expired transfer ticket".into()))?;
    let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cloud_uploads WHERE stage_key=$1 AND state='created' AND expires_at>extract(epoch FROM now())::bigint)")
        .bind(&ticket.key).fetch_one(&state.db).await?;
    if !valid {
        return Err(Error(
            StatusCode::CONFLICT,
            "Upload is no longer accepting data".into(),
        ));
    }
    let path = state.stage_path(&ticket.key)?;
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    if state.ingest_dir.is_some() {
        let max = std::env::var("AUDEXIS_INGEST_MAX_BYTES")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(20_000_000_000);
        let mut entries = tokio::fs::read_dir(path.parent().unwrap()).await?;
        let mut size = 0u64;
        while let Some(entry) = entries.next_entry().await? {
            size = size.saturating_add(entry.metadata().await?.len());
        }
        if size.saturating_add(ticket.size as u64) > max {
            return Err(Error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Server upload cache is full. Transfers will retry shortly.".into(),
            ));
        }
    }
    let temp = path.with_extension(uuid::Uuid::new_v4().to_string());
    let mut file = tokio::fs::File::create(&temp).await?;
    let mut body = request.into_body();
    use http_body_util::BodyExt;
    let mut bytes = 0_i64;
    use sha2::{Digest, Sha256};
    let mut digest = Sha256::new();
    let result = tokio::time::timeout(Duration::from_secs(300), async {
        while let Some(frame) = body.frame().await {
            let frame = frame.map_err(|_| Error::bad("Upload interrupted"))?;
            if let Ok(chunk) = frame.into_data() {
                bytes += chunk.len() as i64;
                if bytes > ticket.size || bytes > state.max_file {
                    return Err(Error::bad("Upload size exceeded"));
                }
                file.write_all(&chunk).await?;
                digest.update(&chunk);
            }
        }
        if bytes != ticket.size {
            return Err(Error::bad("Incomplete upload"));
        }
        if ticket
            .expected_hash
            .as_ref()
            .is_some_and(|expected| *expected != format!("{:x}", digest.finalize()))
        {
            return Err(Error::bad("Uploaded file SHA-256 does not match"));
        }
        file.flush().await?;
        Ok::<(), Error>(())
    })
    .await
    .map_err(|_| Error(StatusCode::REQUEST_TIMEOUT, "Upload timed out".into()))
    .and_then(|r| r);
    drop(file);
    let result = match result {
        Ok(()) => async {
            let mut tx = state.db.begin().await?;
            let valid: Option<String> = sqlx::query_scalar("SELECT id FROM cloud_uploads WHERE stage_key=$1 AND state='created' AND expires_at>extract(epoch FROM now())::bigint FOR UPDATE")
                .bind(&ticket.key).fetch_optional(&mut *tx).await?;
            if valid.is_none() {
                return Err(Error(StatusCode::CONFLICT, "Upload is no longer accepting data".into()));
            }
            tokio::fs::rename(&temp, &path).await?;
            tx.commit().await?;
            Ok(StatusCode::NO_CONTENT)
        }.await,
        Err(error) => Err(error),
    };
    if result.is_err() {
        let _ = tokio::fs::remove_file(temp).await;
    }
    result
}
pub(super) async fn local_download(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let tickets = state.tickets.lock().await;
    let key = tickets
        .get(&token)
        .filter(|t| !t.write && t.expires > Instant::now())
        .map(|t| t.key.clone())
        .ok_or_else(|| Error(StatusCode::UNAUTHORIZED, "Expired transfer ticket".into()))?;
    drop(tickets);
    let mut file = tokio::fs::File::open(state.storage.path(&key)?).await?;
    let size = file.metadata().await?.len();
    let range = headers.get(header::RANGE);
    let (start, length) = if let Some(range) = range {
        match range
            .to_str()
            .ok()
            .and_then(|range| byte_range(range, size))
        {
            Some((start, end)) => (start, end - start + 1),
            None => {
                return Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(header::CONTENT_RANGE, format!("bytes */{size}"))
                    .body(Body::empty())
                    .map_err(|_| Error::bad("Could not serve media"));
            }
        }
    } else {
        (0, size)
    };
    file.seek(std::io::SeekFrom::Start(start)).await?;
    let mut response = Response::builder()
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, length)
        .header(
            header::CONTENT_TYPE,
            if key.starts_with("artwork/") {
                "image/png"
            } else {
                "application/octet-stream"
            },
        );
    if range.is_some() {
        response = response.status(StatusCode::PARTIAL_CONTENT).header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{}/{size}", start + length - 1),
        );
    }
    response
        .body(Body::from_stream(ReaderStream::new(file.take(length))))
        .map_err(|_| Error::bad("Could not serve media"))
}
pub(super) fn byte_range(value: &str, size: u64) -> Option<(u64, u64)> {
    if size == 0 {
        return None;
    }
    let (start, end) = value.strip_prefix("bytes=")?.split_once('-')?;
    if start.is_empty() {
        let suffix: u64 = end.parse().ok()?;
        return (suffix > 0).then(|| (size.saturating_sub(suffix), size - 1));
    }
    let start: u64 = start.parse().ok()?;
    let end = if end.is_empty() {
        size - 1
    } else {
        end.parse::<u64>().ok()?.min(size - 1)
    };
    (start < size && start <= end).then_some((start, end))
}
