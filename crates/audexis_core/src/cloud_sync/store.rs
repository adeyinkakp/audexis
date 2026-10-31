mod playlists;
pub(crate) use playlists::reconcile_playlists;

mod outbox;
pub use outbox::*;

mod discover;
pub use discover::*;

mod apply;
pub use apply::*;

use super::{
    client::{Result, SyncError},
    protocol::*,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqliteConnection, SqlitePool};

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "job", rename_all = "snake_case")]
pub enum Job {
    Upload {
        file_id: i64,
        request: UploadRequest,
        upload_id: Option<String>,
        uploaded: bool,
    },
    Mutate {
        file_id: Option<i64>,
        playlist_id: Option<i64>,
        request: Mutation,
    },
}
#[derive(Clone, sqlx::FromRow)]
pub struct Pending {
    pub operation_id: String,
    pub payload: String,
    pub attempts: i64,
}
pub async fn local_metadata(conn: &mut SqliteConnection, file_id: i64) -> Result<Metadata> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key,value FROM metadata_texts WHERE file_id=? ORDER BY key,ord")
            .bind(file_id)
            .fetch_all(&mut *conn)
            .await?;
    let mut metadata = Metadata::default();
    for (key, value) in rows {
        metadata.tags.entry(key).or_default().push(value)
    }
    metadata.loved = sqlx::query_scalar::<_, bool>(
        "SELECT COALESCE((SELECT loved FROM media_info WHERE file_id=?),0)",
    )
    .bind(file_id)
    .fetch_one(&mut *conn)
    .await?;
    Ok(metadata.for_sync())
}
pub async fn enqueue(pool: &SqlitePool, connection: &str, key: &str, job: &Job) -> Result<()> {
    insert_job(pool, connection, key, job, false).await
}
pub async fn enqueue_user(pool: &SqlitePool, connection: &str, key: &str, job: &Job) -> Result<()> {
    insert_job(pool, connection, key, job, true).await
}
async fn insert_job(
    pool: &SqlitePool,
    connection: &str,
    key: &str,
    job: &Job,
    strict: bool,
) -> Result<()> {
    let op = match job {
        Job::Upload { request, .. } => &request.operation_id,
        Job::Mutate { request, .. } => &request.operation_id,
    };
    let result=sqlx::query("INSERT INTO cloud_outbox(operation_id,connection_id,entity_key,payload) VALUES (?,?,?,?) ON CONFLICT(connection_id,entity_key) DO NOTHING").bind(op).bind(connection).bind(key).bind(serde_json::to_string(job)?).execute(pool).await?;
    if strict && result.rows_affected() == 0 {
        return Err(SyncError::local(
            "A change for this item is already queued. Sync or discard it before editing again.",
        ));
    }
    Ok(())
}
pub async fn write_metadata(
    conn: &mut SqliteConnection,
    id: i64,
    metadata: &Metadata,
) -> Result<()> {
    sqlx::query("DELETE FROM metadata_texts WHERE file_id=?")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    for (key, values) in &metadata.tags {
        for (ord, value) in values.iter().enumerate() {
            sqlx::query("INSERT INTO metadata_texts(file_id,key,value,ord) VALUES (?,?,?,?)")
                .bind(id)
                .bind(key)
                .bind(value)
                .bind(ord as i64)
                .execute(&mut *conn)
                .await?;
        }
    }
    sqlx::query("INSERT INTO media_info(file_id,loved) VALUES (?,?) ON CONFLICT(file_id) DO UPDATE SET loved=excluded.loved").bind(id).bind(metadata.loved).execute(&mut *conn).await?;
    Ok(())
}
