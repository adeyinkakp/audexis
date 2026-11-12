use super::*;

pub async fn run(state: Arc<AppState>) {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
    let mut extended_until = 0;
    let mut last_cleanup = tokio::time::Instant::now() - std::time::Duration::from_secs(60);
    loop {
        tokio::select! {_=tick.tick()=>{},_=state.processing.notified()=>{}}
        if crate::storage::cooldown_error().is_some() {
            let deadline =
                chrono::Utc::now().timestamp() + crate::storage::cooldown_seconds() as i64;
            if deadline > extended_until {
                if sqlx::query("UPDATE cloud_uploads SET expires_at=GREATEST(expires_at,$1+3600) WHERE state IN ('created','uploaded','processing')")
                    .bind(deadline).execute(&state.db).await.is_ok() { extended_until = deadline; }
            }
            continue;
        }
        for _ in 0..10 {
            let claimed=sqlx::query("UPDATE cloud_uploads SET state='processing',lease_at=extract(epoch FROM now())::bigint WHERE id=(SELECT id FROM cloud_uploads WHERE ((state='uploaded' AND lease_at<=extract(epoch FROM now())::bigint) OR (state='processing' AND lease_at<extract(epoch FROM now())::bigint-600)) AND expires_at>extract(epoch FROM now())::bigint ORDER BY expires_at FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING id,user_id,stage_key,request,error")
   .fetch_optional(&state.db).await;
            let row = match claimed {
                Ok(Some(row)) => row,
                Ok(None) => break,
                Err(error) => {
                    tracing::error!(%error,"Cannot claim upload");
                    break;
                }
            };
            let id: String = row.get("id");
            let user: String = row.get("user_id");
            let stage: String = row.get("stage_key");
            let request: UploadRequest = match serde_json::from_value(row.get("request")) {
                Ok(r) => r,
                Err(_) => {
                    let _=sqlx::query("UPDATE cloud_uploads SET state='failed',error='Invalid upload request' WHERE id=$1").bind(&id).execute(&state.db).await;
                    continue;
                }
            };
            let attempt = row
                .get::<Option<String>, _>("error")
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|value| value.get("processing_attempt").and_then(|v| v.as_u64()))
                .unwrap_or(0)
                + 1;
            let processing = process(&state, &id, &user, &stage, request);
            tokio::pin!(processing);
            let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(30));
            heartbeat.tick().await;
            let result = loop {
                tokio::select! {
                    result=&mut processing=>break result,
                    _=heartbeat.tick()=>{
                        if let Err(error)=sqlx::query("UPDATE cloud_uploads SET lease_at=extract(epoch FROM now())::bigint WHERE id=$1 AND state='processing'").bind(&id).execute(&state.db).await {
                            tracing::warn!(upload_id=%id,%error,"Could not renew processing lease");
                        }
                    }
                }
            };
            match result {
                Ok(()) => {
                    let _ = state.delete_stage(&stage).await;
                }
                Err(mut error) => {
                    if crate::storage::cooldown_error().is_some() {
                        let wait = crate::storage::cooldown_seconds() as i64;
                        let saved = serde_json::json!({"status":503,"message":error.1,"processing_attempt":attempt.saturating_sub(1)}).to_string();
                        let _=sqlx::query("UPDATE cloud_uploads SET state='uploaded',lease_at=extract(epoch FROM now())::bigint+$2,expires_at=GREATEST(expires_at,extract(epoch FROM now())::bigint+$2+3600),error=$3 WHERE id=$1 AND state='processing'")
                            .bind(&id).bind(wait).bind(saved).execute(&state.db).await;
                        break;
                    }
                    let transient = error.0.is_server_error();
                    let retry = transient && attempt < 3;
                    if transient && !retry {
                        error.0 = StatusCode::FAILED_DEPENDENCY;
                        error.1 = format!(
                            "{}; processing retries exhausted. The temporary upload is retained until expiry.",
                            error.1
                        );
                    }

                    tracing::warn!(upload_id=%id,status=%error.0,reason=%error.1,attempt,retrying=retry,"Upload processing failed");
                    let saved=    serde_json::json!({"status":error.0.as_u16(),"message":error.1,"processing_attempt":attempt}).to_string();
                    if retry {
                        let delay = 15_i64 * attempt as i64;
                        let _=sqlx::query("UPDATE cloud_uploads SET state='uploaded',lease_at=extract(epoch FROM now())::bigint+$3,error=$2 WHERE id=$1 AND state='processing'").bind(&id).bind(saved).bind(delay).execute(&state.db).await;
                    } else {
                        let _=sqlx::query("UPDATE cloud_uploads SET state='failed',error=$2 WHERE id=$1 AND state='processing'").bind(&id).bind(saved).execute(&state.db).await;
                    }
                    if !retry && !transient {
                        let _ = state.delete_stage(&stage).await;
                    }
                }
            }
        }
        if last_cleanup.elapsed() >= std::time::Duration::from_secs(60) {
            if let Err(error) = cleanup(&state).await {
                tracing::warn!(status=%error.0,"Cloud cleanup failed")
            }
            last_cleanup = tokio::time::Instant::now();
        }
    }
}
