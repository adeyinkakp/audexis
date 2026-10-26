mod media;
pub use media::*;

mod mutations;
pub use mutations::*;

mod uploads;
pub use uploads::*;

use crate::{
    AppState,
    auth::Account,
    db,
    error::{Error, Result},
    storage::Storage,
};
use audexis_core::cloud_sync::protocol::*;
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct Cursor {
    #[serde(default)]
    after: i64,
}
pub async fn session(Extension(account): Extension<Account>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"authenticated":true,"user_id":account.user_id}))
}
pub async fn usage(
    State(state): State<Arc<AppState>>,
    Extension(account): Extension<Account>,
) -> Result<Json<Usage>> {
    let used:i64=sqlx::query_scalar("SELECT COALESCE(sum(o.size),0)::bigint FROM cloud_tracks t JOIN cloud_objects o ON o.sha256=t.object_hash WHERE t.user_id=$1 AND NOT t.deleted").bind(&account.user_id).fetch_one(&state.db).await?;
    let reserved:i64=sqlx::query_scalar("SELECT COALESCE(sum(size),0)::bigint FROM cloud_uploads WHERE user_id=$1 AND state IN ('created','uploaded','processing') AND expires_at>extract(epoch FROM now())::bigint").bind(&account.user_id).fetch_one(&state.db).await?;
    Ok(Json(Usage {
        used_bytes: used,
        reserved_bytes: reserved,
        quota_bytes: account.quota,
    }))
}
pub async fn changes(
    State(state): State<Arc<AppState>>,
    Extension(account): Extension<Account>,
    Query(cursor): Query<Cursor>,
) -> Result<Json<Changes>> {
    if cursor.after < 0 {
        return Err(Error::bad("Invalid sync cursor"));
    }
    let rows = sqlx::query(
        "WITH candidates AS (SELECT seq,value FROM cloud_changes WHERE user_id=$1 AND seq>$2 ORDER BY seq LIMIT 200), bounded AS (SELECT seq,value,sum(octet_length(value::text)+64) OVER (ORDER BY seq) AS bytes FROM candidates), page AS (SELECT seq,value FROM bounded WHERE bytes<=$3) SELECT p.seq,p.value,EXISTS(SELECT 1 FROM cloud_changes WHERE user_id=$1 AND seq>COALESCE((SELECT max(seq) FROM page),$2)) AS has_more FROM (VALUES (1)) AS anchor(dummy) LEFT JOIN page p ON TRUE ORDER BY p.seq",
    )
    .bind(&account.user_id)
    .bind(cursor.after)
    .bind(4_194_048_i64 - 256)
    .fetch_all(&state.db)
    .await?;
    let has_more: bool = rows.first().map(|row| row.get("has_more")).unwrap_or(false);
    let mut changes = Vec::new();
    for row in rows {
        if let Some(cursor) = row.get::<Option<i64>, _>("seq") {
            changes.push(Change {
                cursor,
                value: serde_json::from_value(row.get("value"))?,
            });
        }
    }
    let next_cursor = changes.last().map(|c| c.cursor).unwrap_or(cursor.after);
    if has_more && changes.is_empty() {
        return Err(Error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "A sync document exceeds the response size limit".into(),
        ));
    }
    Ok(Json(Changes {
        changes,
        next_cursor,
        has_more,
    }))
}
fn check_metadata(metadata: &Metadata) -> Result<()> {
    metadata.validate().map_err(Error::bad)
}

fn check_operation(id: &str) -> Result<()> {
    if uuid::Uuid::parse_str(id).is_err() {
        return Err(Error::bad("Invalid operation ID"));
    }
    Ok(())
}
