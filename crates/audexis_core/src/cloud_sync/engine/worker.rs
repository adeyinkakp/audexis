use super::*;

pub async fn cycle(
    context: &CloudSyncContext,
    upload_local: bool,
    process_jobs: bool,
    progress: &tokio::sync::watch::Sender<super::super::CloudSyncProgress>,
) -> Result<()> {
    if uuid::Uuid::parse_str(&context.connection_id).is_err() {
        return Err(SyncError::local(
            "A verified connection ID is required for sync",
        ));
    }
    progress.send_modify(|p| {
        p.phase = super::super::SyncPhase::Checking;
        p.file_name = None;
        p.bytes_sent = 0;
        p.bytes_total = 0;
    });
    let client = context.client()?;
    client.verify(&context.server_user_id).await?;
    pull(context, &client).await?;
    context.refresh_library().await?;
    if upload_local || process_jobs {
        progress.send_modify(|p| p.phase = super::super::SyncPhase::Discovering);
        store::discover_with_uploads(&context.db.pool, &context.connection_id, upload_local)
            .await?;
    }
    store::recover_metadata_limits(&context.db.pool, &context.connection_id).await?;
    let (queued,failed):(i64,i64)=sqlx::query_as("SELECT count(*),COALESCE(sum(CASE WHEN error IS NOT NULL THEN 1 ELSE 0 END),0) FROM cloud_outbox WHERE connection_id=?").bind(&context.connection_id).fetch_one(&context.db.pool).await?;
    progress.send_modify(|p| {
        if p.queued == 0 && queued > 0 {
            p.completed = 0;
        }
        p.queued = queued as u64;
        p.failed = failed as u64;
    });
    if !process_jobs {
        progress.send_modify(|p| {
            p.phase = if p.queued == 0 {
                super::super::SyncPhase::Complete
            } else {
                super::super::SyncPhase::Waiting
            }
        });
        return thumbnails(context, &client).await;
    }
    let jobs = store::pending(&context.db.pool, &context.connection_id, upload_local).await?;
    let mut first_error = None;
    for job in jobs {
        let payload: Job = serde_json::from_str(&job.payload)?;
        if matches!(
            payload,
            Job::Upload {
                upload_id: None,
                ..
            }
        ) {
            let active:i64=sqlx::query_scalar("SELECT count(*) FROM cloud_outbox WHERE connection_id=? AND state='pending' AND json_extract(payload,'$.job')='upload' AND json_extract(payload,'$.upload_id') IS NOT NULL").bind(&context.connection_id).fetch_one(&context.db.pool).await?;
            if active >= 3 {
                continue;
            }
        }
        sqlx::query("UPDATE cloud_outbox SET error=NULL WHERE connection_id=? AND operation_id=?")
            .bind(&context.connection_id)
            .bind(&job.operation_id)
            .execute(&context.db.pool)
            .await?;
        match process(context, &client, &job, progress).await {
            Ok(()) => {
                let remains:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cloud_outbox WHERE connection_id=? AND operation_id=?)").bind(&context.connection_id).bind(&job.operation_id).fetch_one(&context.db.pool).await?;
                let count: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM cloud_outbox WHERE connection_id=?")
                        .bind(&context.connection_id)
                        .fetch_one(&context.db.pool)
                        .await?;
                progress.send_modify(|p| {
                    if !remains && (count as u64) < p.queued {
                        p.completed += p.queued - count as u64;
                    }
                    p.queued = count as u64;
                });
            }
            Err(error) => {
                if error.status == Some(429) || (error.status == Some(503) && error.retry_after > 0)
                {
                    store::defer_busy(
                        &context.db.pool,
                        &context.connection_id,
                        &job.operation_id,
                        error.retry_after.max(3),
                    )
                    .await?;
                    return Err(error);
                }
                if matches!(error.status, Some(400 | 404 | 405 | 413 | 415 | 422 | 424)) {
                    if let Some(id)=sqlx::query_scalar::<_,Option<String>>("SELECT json_extract(payload,'$.upload_id') FROM cloud_outbox WHERE connection_id=? AND operation_id=?").bind(&context.connection_id).bind(&job.operation_id).fetch_optional(&context.db.pool).await?.flatten() {
                        let _=client.cancel_upload(&id).await;
                    }
                }
                store::fail(&context.db.pool, &context.connection_id, &job, &error).await?;
                let failed: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM cloud_outbox WHERE connection_id=? AND error IS NOT NULL",
                )
                .bind(&context.connection_id)
                .fetch_one(&context.db.pool)
                .await?;
                progress.send_modify(|p| {
                    p.failed = failed as u64;
                    p.phase = super::super::SyncPhase::Waiting;
                    p.file_name = None;
                    p.bytes_sent = 0;
                    p.bytes_total = 0;
                });
                if matches!(error.status, Some(401 | 403)) {
                    return Err(error);
                }
                if first_error.is_none() {
                    first_error = Some(error)
                }
            }
        }
    }
    pull(context, &client).await?;
    let conflicts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM cloud_outbox WHERE connection_id=? AND state='conflict'",
    )
    .bind(&context.connection_id)
    .fetch_one(&context.db.pool)
    .await?;
    if conflicts > 0 {
        return Err(SyncError::local(format!(
            "{conflicts} changes need conflict resolution in Cloud Sync"
        )));
    }
    let failed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM cloud_outbox WHERE connection_id=? AND error IS NOT NULL",
    )
    .bind(&context.connection_id)
    .fetch_one(&context.db.pool)
    .await?;
    progress.send_modify(|p| {
        p.failed = failed as u64;
        p.phase = if p.queued == 0 {
            super::super::SyncPhase::Complete
        } else if p.phase == super::super::SyncPhase::Processing {
            super::super::SyncPhase::Processing
        } else {
            super::super::SyncPhase::Waiting
        };
        if p.phase != super::super::SyncPhase::Processing {
            p.file_name = None;
        }
        p.bytes_sent = 0;
        p.bytes_total = 0;
    });
    if let Some(error) = first_error {
        return Err(error);
    }
    context.refresh_library().await?;
    thumbnails(context, &client).await
}
