use super::*;

pub async fn start_upload(
    State(state): State<Arc<AppState>>,
    Extension(account): Extension<Account>,
    Json(request): Json<UploadRequest>,
) -> Result<Json<Upload>> {
    check_operation(&request.operation_id)?;
    check_metadata(&request.metadata)?;
    if request.size <= 0
        || request.size > state.max_file
        || request.sha256.len() != 64
        || !request
            .sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || request.file_name.is_empty()
        || request.file_name.len() > 512
    {
        return Err(Error::bad("Invalid upload size, hash, or file name"));
    }
    let mut tx = state.db.begin().await?;
    let quota = db::lock_account(&mut tx, &account.user_id).await?;
    let previous: Option<(String, serde_json::Value)> =
        sqlx::query_as("SELECT id,request FROM cloud_uploads WHERE user_id=$1 AND operation_id=$2")
            .bind(&account.user_id)
            .bind(&request.operation_id)
            .fetch_optional(&mut *tx)
            .await?;
    if let Some((id, old)) = previous {
        if old != serde_json::to_value(&request)? {
            return Err(Error(
                StatusCode::CONFLICT,
                "Operation ID already used with different upload data".into(),
            ));
        }
        tx.commit().await?;
        return upload_reply(&state, &account, &id).await.map(Json);
    }
    let existing: Option<(String, serde_json::Value)> = sqlx::query_as(
        "SELECT id,document FROM cloud_tracks WHERE user_id=$1 AND object_hash=$2 AND NOT deleted",
    )
    .bind(&account.user_id)
    .bind(&request.sha256)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((id, document)) = existing {
        let track: Track = serde_json::from_value(document)?;
        if track.size == request.size {
            tx.commit().await?;
            return Ok(Json(Upload {
                id,
                state: "ready".into(),
                upload_url: None,
                track: Some(track),
                error: None,
                error_status: None,
            }));
        }
    }
    let (used,reserved,pending):(i64,i64,i64)=sqlx::query_as("SELECT (SELECT COALESCE(sum(o.size),0)::bigint FROM cloud_tracks t JOIN cloud_objects o ON o.sha256=t.object_hash WHERE t.user_id=$1 AND NOT t.deleted),(SELECT COALESCE(sum(size),0)::bigint FROM cloud_uploads WHERE user_id=$1 AND state IN ('created','uploaded','processing') AND expires_at>extract(epoch FROM now())::bigint),(SELECT count(*) FROM cloud_uploads WHERE user_id=$1 AND state IN ('created','uploaded','processing') AND expires_at>extract(epoch FROM now())::bigint)").bind(&account.user_id).fetch_one(&mut *tx).await?;
    if pending >= 3 {
        return Err(Error(
            StatusCode::TOO_MANY_REQUESTS,
            "Three uploads are already pending".into(),
        ));
    }
    if used.saturating_add(reserved).saturating_add(request.size) > quota {
        return Err(Error(
            StatusCode::INSUFFICIENT_STORAGE,
            "Cloud storage quota exceeded".into(),
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let stage = format!("staging/{id}");
    sqlx::query("INSERT INTO cloud_uploads(id,user_id,operation_id,request,stage_key,size,expires_at) VALUES ($1,$2,$3,$4,$5,$6,extract(epoch FROM now())::bigint+3600)").bind(&id).bind(&account.user_id).bind(&request.operation_id).bind(serde_json::to_value(&request)?).bind(&stage).bind(request.size).execute(&mut *tx).await?;
    if state.ingest_dir.is_none() {
        sqlx::query("INSERT INTO cloud_object_writes(object_key,created_at) VALUES ($1,extract(epoch FROM now())::bigint+900)").bind(&stage).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    upload_reply(&state, &account, &id).await.map(Json)
}
pub async fn upload_reply(state: &AppState, account: &Account, id: &str) -> Result<Upload> {
    let row=sqlx::query("SELECT state,stage_key,size,track_id,error,expires_at,request->>'sha256' AS sha256 FROM cloud_uploads WHERE id=$1 AND user_id=$2").bind(id).bind(&account.user_id).fetch_optional(&state.db).await?.ok_or_else(||Error(StatusCode::NOT_FOUND,"Upload not found".into()))?;
    let status: String = row.get("state");
    if status != "ready" {
        if let Some(error) = crate::storage::cooldown_error() {
            return Err(error);
        }
    }
    let expires: i64 = row.get("expires_at");
    let key: String = row.get("stage_key");
    let size: i64 = row.get("size");
    let expired = expires <= chrono::Utc::now().timestamp() && status == "created";
    let url = if status == "created" && !expired {
        Some(if state.ingest_dir.is_some() {
            let origin = std::env::var("AUDEXIS_PUBLIC_ORIGIN")
                .map_err(|_| Error::bad("AUDEXIS_PUBLIC_ORIGIN is required for server uploads"))?;
            let ticket = state.ticket(key.clone(), true, size).await;
            if let Some(saved) = state.tickets.lock().await.get_mut(&ticket) {
                saved.expected_hash = row.get("sha256");
            }
            format!("{}/transfer/{ticket}", origin.trim_end_matches('/'))
        } else {
            match &state.storage {
                Storage::B2 { .. } => state.storage.signed("PUT", &key, Some(size))?,
                Storage::Local { origin, .. } => {
                    let ticket = state.ticket(key, true, size).await;
                    format!("{origin}/transfer/{ticket}")
                }
            }
        })
    } else {
        None
    };
    let track_id: Option<String> = row.get("track_id");
    let track = if let Some(track_id) = track_id {
        let value: Option<serde_json::Value> =
            sqlx::query_scalar("SELECT document FROM cloud_tracks WHERE id=$1 AND user_id=$2")
                .bind(track_id)
                .bind(&account.user_id)
                .fetch_optional(&state.db)
                .await?;
        value.map(serde_json::from_value).transpose()?
    } else {
        None
    };
    let saved: Option<String> = row.get("error");
    let structured = saved
        .as_ref()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
    let error_status = structured
        .as_ref()
        .and_then(|s| s.get("status"))
        .and_then(|s| s.as_u64())
        .and_then(|s| u16::try_from(s).ok());
    let error = structured
        .as_ref()
        .and_then(|s| s.get("message"))
        .and_then(|s| s.as_str())
        .map(str::to_owned)
        .or(saved);
    Ok(Upload {
        id: id.into(),
        state: if expired { "expired".into() } else { status },
        upload_url: url,
        track,
        error,
        error_status,
    })
}
pub async fn get_upload(
    State(state): State<Arc<AppState>>,
    Extension(account): Extension<Account>,
    Path(id): Path<String>,
) -> Result<Json<Upload>> {
    upload_reply(&state, &account, &id).await.map(Json)
}
pub async fn finish_upload(
    State(state): State<Arc<AppState>>,
    Extension(account): Extension<Account>,
    Path(id): Path<String>,
) -> Result<Json<Upload>> {
    let key: Option<String> = sqlx::query_scalar("UPDATE cloud_uploads SET state='uploaded' WHERE id=$1 AND user_id=$2 AND state='created' AND expires_at>extract(epoch FROM now())::bigint RETURNING stage_key").bind(&id).bind(&account.user_id).fetch_optional(&state.db).await?;
    if let Some(key) = key {
        state.revoke_upload_tickets(&key).await;
    }
    state.processing.notify_one();
    upload_reply(&state, &account, &id).await.map(Json)
}
pub async fn cancel_upload(
    State(state): State<Arc<AppState>>,
    Extension(account): Extension<Account>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    let key:Option<String>=sqlx::query_scalar("UPDATE cloud_uploads SET state='cancelled' WHERE id=$1 AND user_id=$2 AND state IN ('created','uploaded') RETURNING stage_key").bind(id).bind(account.user_id).fetch_optional(&state.db).await?;
    if let Some(key) = key {
        state.revoke_upload_tickets(&key).await;
        let _ = state.delete_stage(&key).await;
    }
    Ok(StatusCode::NO_CONTENT)
}
