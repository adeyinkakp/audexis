use super::*;

pub async fn pending(pool: &SqlitePool, connection: &str, uploads: bool) -> Result<Vec<Pending>> {
    Ok(sqlx::query_as("SELECT operation_id,payload,attempts FROM cloud_outbox WHERE connection_id=? AND state='pending' AND retry_at<=unixepoch() AND (? OR json_extract(payload,'$.job')!='upload') ORDER BY CASE WHEN json_extract(payload,'$.job')='upload' AND json_extract(payload,'$.upload_id') IS NOT NULL THEN 0 WHEN json_extract(payload,'$.job')='mutate' THEN 1 ELSE 2 END,rowid LIMIT 10").bind(connection).bind(uploads).fetch_all(pool).await?)
}
pub async fn fail(
    pool: &SqlitePool,
    connection: &str,
    job: &Pending,
    error: &SyncError,
) -> Result<()> {
    let conflict = error.status == Some(409);
    let invalid = matches!(error.status, Some(400 | 404 | 405 | 413 | 415 | 422 | 424));
    let wait = error.retry_after.max(
        (5_i64.saturating_mul(
            1_i64
                .checked_shl(job.attempts.min(10) as u32)
                .unwrap_or(1024),
        ))
        .min(3600) as u64,
    );
    sqlx::query("UPDATE cloud_outbox SET attempts=attempts+1,retry_at=unixepoch()+?,state=?,error=? WHERE connection_id=? AND operation_id=?").bind(wait as i64).bind(if conflict{"conflict"}else if invalid{"failed"}else{"pending"}).bind(&error.message).bind(connection).bind(&job.operation_id).execute(pool).await?;
    Ok(())
}

pub async fn defer_busy(
    pool: &SqlitePool,
    connection: &str,
    operation: &str,
    seconds: u64,
) -> Result<()> {
    sqlx::query("UPDATE cloud_outbox SET retry_at=unixepoch()+?,error=NULL WHERE connection_id=? AND operation_id=?").bind(seconds.clamp(1,3600) as i64).bind(connection).bind(operation).execute(pool).await?;
    Ok(())
}

pub async fn recover_metadata_limits(pool: &SqlitePool, connection: &str) -> Result<()> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT operation_id,payload FROM cloud_outbox WHERE connection_id=? AND state='failed' AND error LIKE '%Metadata exceeds allowed limits%' AND COALESCE(json_extract(payload,'$.metadata_limits_version'),0)<1"
    ).bind(connection).fetch_all(pool).await?;
    for (old_id, payload) in rows {
        let mut job: Job = serde_json::from_str(&payload)?;
        let new_id = uuid::Uuid::new_v4().to_string();
        let metadata = match &mut job {
            Job::Upload {
                request,
                upload_id: None,
                uploaded: false,
                ..
            } => {
                request.operation_id = new_id.clone();
                &mut request.metadata
            }
            Job::Mutate {
                request:
                    Mutation {
                        operation_id,
                        edit: Edit::Track { metadata, .. },
                        ..
                    },
                ..
            } => {
                *operation_id = new_id.clone();
                metadata
            }
            _ => continue,
        };
        *metadata = metadata.for_sync();
        if metadata.validate().is_err() {
            continue;
        }
        let mut value = serde_json::to_value(&job)?;
        value["metadata_limits_version"] = serde_json::json!(1);
        sqlx::query("UPDATE cloud_outbox SET operation_id=?,payload=?,state='pending',attempts=0,retry_at=0,error=NULL WHERE connection_id=? AND operation_id=? AND state='failed'")
            .bind(new_id).bind(serde_json::to_string(&value)?).bind(connection).bind(old_id).execute(pool).await?;
    }
    Ok(())
}
