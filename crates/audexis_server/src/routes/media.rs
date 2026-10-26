use super::*;

#[derive(Deserialize, Default)]
pub struct MediaQuery {
    quality: Option<String>,
}
pub async fn media(
    State(state): State<Arc<AppState>>,
    Extension(account): Extension<Account>,
    Path((id, kind)): Path<(String, String)>,
    Query(query): Query<MediaQuery>,
) -> Result<Json<MediaUrl>> {
    let row=sqlx::query("SELECT o.object_key,o.artwork_key,o.artwork_size,o.artwork_sha256,o.size,o.sha256 FROM cloud_tracks t JOIN cloud_objects o ON o.sha256=t.object_hash WHERE t.id=$1 AND t.user_id=$2 AND NOT t.deleted").bind(id).bind(account.user_id).fetch_optional(&state.db).await?.ok_or_else(||Error(StatusCode::NOT_FOUND,"Track not found".into()))?;
    let mut size: i64 = row.get("size");
    let mut hash: String = row.get("sha256");
    let mut key: String = match kind.as_str() {
        "audio" => row.get("object_key"),
        "artwork" => row
            .get::<Option<String>, _>("artwork_key")
            .ok_or_else(|| Error(StatusCode::NOT_FOUND, "No artwork".into()))?,
        _ => return Err(Error::bad("Unknown media type")),
    };
    if kind == "artwork" {
        (size, hash) = match (
            row.get::<Option<i64>, _>("artwork_size"),
            row.get::<Option<String>, _>("artwork_sha256"),
        ) {
            (Some(size), Some(hash)) => (size, hash),
            _ => {
                let (size, hash) = crate::ingest::artwork_descriptor(&state, &key).await?;
                sqlx::query("UPDATE cloud_objects SET artwork_size=$1,artwork_sha256=$2 WHERE artwork_key=$3")
                    .bind(size).bind(&hash).bind(&key).execute(&state.db).await?;
                (size, hash)
            }
        };
    }
    let requested = query.quality.as_deref().unwrap_or("original");
    if !crate::variants::quality(requested) {
        return Err(Error::bad("Unknown streaming quality"));
    }
    let mut quality = "original".to_string();
    let mut preparing = false;
    if kind == "audio" && requested != "original" && state.ffmpeg.is_some() {
        sqlx::query("INSERT INTO cloud_media_variants(object_hash,quality) VALUES ($1,$2) ON CONFLICT(object_hash,quality) DO UPDATE SET state='queued',attempts=0 WHERE cloud_media_variants.state='failed' AND cloud_media_variants.retry_at<=extract(epoch FROM now())::bigint")
            .bind(&hash).bind(requested).execute(&state.db).await?;
        if let Some(variant) = sqlx::query("SELECT state,object_key,size,sha256 FROM cloud_media_variants WHERE object_hash=$1 AND quality=$2")
            .bind(&hash).bind(requested).fetch_optional(&state.db).await? {
            let status: String = variant.get("state");
            if status == "ready" {
                key = variant.get("object_key"); size = variant.get("size"); hash = variant.get("sha256"); quality = requested.into();
            } else { preparing = matches!(status.as_str(), "queued" | "processing"); }
        }
    }
    let url = match &state.storage {
        Storage::B2 { .. } => state.storage.signed("GET", &key, None)?,
        Storage::Local { origin, .. } => {
            let ticket = state.ticket(key, false, size).await;
            format!("{origin}/transfer/{ticket}")
        }
    };
    Ok(Json(MediaUrl {
        url,
        sha256: hash,
        size,
        quality,
        preparing,
    }))
}
