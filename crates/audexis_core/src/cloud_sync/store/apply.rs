use super::*;

pub async fn apply(
    pool: &SqlitePool,
    connection: &str,
    changes: &[Change],
    cursor: Option<i64>,
) -> Result<()> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    apply_to_connection(&mut tx, connection, changes, cursor).await?;
    tx.commit().await?;
    Ok(())
}

/// Apply a sync page inside the caller's transaction so client state can commit with it.
pub async fn apply_to_connection(
    tx: &mut SqliteConnection,
    connection: &str,
    changes: &[Change],
    cursor: Option<i64>,
) -> Result<()> {
    for change in changes {
        match &change.value {
            ChangeValue::Track(track) => {
                let previous: Option<i64> = sqlx::query_scalar(
                    "SELECT revision FROM cloud_tracks WHERE connection_id=? AND id=?",
                )
                .bind(connection)
                .bind(&track.id)
                .fetch_optional(&mut *tx)
                .await?;
                if previous.is_some_and(|rev| rev > track.revision) {
                    continue;
                }
                sqlx::query("INSERT INTO cloud_tracks(connection_id,id,revision,document) VALUES (?,?,?,?) ON CONFLICT(connection_id,id) DO UPDATE SET revision=excluded.revision,document=excluded.document").bind(connection).bind(&track.id).bind(track.revision).bind(serde_json::to_string(track)?).execute(&mut *tx).await?;
                if !track.deleted {
                    let matches:Vec<i64>=sqlx::query_scalar("SELECT f.id FROM files f WHERE f.cloud_connection_id IS NULL AND f.hash=? AND f.size=? AND NOT EXISTS(SELECT 1 FROM file_cloud_links l WHERE l.file_id=f.id AND l.connection_id=?)").bind(&track.sha256).bind(track.size).bind(connection).fetch_all(&mut *tx).await?;
                    for id in matches {
                        sqlx::query("INSERT INTO file_cloud_links(file_id,connection_id,cloud_track_id) VALUES (?,?,?) ON CONFLICT(file_id,connection_id) DO NOTHING").bind(id).bind(connection).bind(&track.id).execute(&mut *tx).await?;
                    }
                }
                let ids:Vec<i64>=sqlx::query_scalar("SELECT file_id FROM file_cloud_links WHERE connection_id=? AND cloud_track_id=?").bind(connection).bind(&track.id).fetch_all(&mut *tx).await?;
                for id in ids {
                    let baseline:Option<String>=sqlx::query_scalar("SELECT metadata FROM cloud_local_state WHERE connection_id=? AND file_id=?").bind(connection).bind(id).fetch_optional(&mut *tx).await?;
                    let current = serde_json::to_string(&local_metadata(tx, id).await?)?;
                    if !track.deleted && baseline.as_ref().is_none_or(|base| base == &current) {
                        write_metadata(tx, id, &track.metadata).await?;
                    }
                    if baseline.as_ref().is_none_or(|base| base == &current) {
                        sqlx::query("INSERT INTO cloud_local_state(connection_id,file_id,metadata,revision) VALUES (?,?,?,?) ON CONFLICT(connection_id,file_id) DO UPDATE SET metadata=excluded.metadata,revision=excluded.revision").bind(connection).bind(id).bind(serde_json::to_string(&track.metadata)?).bind(track.revision).execute(&mut *tx).await?;
                    }
                }
            }
            ChangeValue::Playlist(playlist) => {
                let previous: Option<i64> = sqlx::query_scalar(
                    "SELECT revision FROM cloud_playlists WHERE connection_id=? AND id=?",
                )
                .bind(connection)
                .bind(&playlist.id)
                .fetch_optional(&mut *tx)
                .await?;
                if previous.is_some_and(|v| v > playlist.revision) {
                    continue;
                }
                sqlx::query("INSERT INTO cloud_playlists(connection_id,id,revision,document) VALUES (?,?,?,?) ON CONFLICT(connection_id,id) DO UPDATE SET revision=excluded.revision,document=excluded.document WHERE excluded.revision>=cloud_playlists.revision").bind(connection).bind(&playlist.id).bind(playlist.revision).bind(serde_json::to_string(playlist)?).execute(&mut *tx).await?;
            }
        }
    }
    if let Some(cursor) = cursor {
        sqlx::query("INSERT INTO cloud_sync_cursors(connection_id,cursor) VALUES (?,?) ON CONFLICT(connection_id) DO UPDATE SET cursor=excluded.cursor").bind(connection).bind(cursor).execute(&mut *tx).await?;
    }
    Ok(())
}
