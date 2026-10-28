use super::{client::Result, protocol::Track, store};
use crate::database::Database;

impl Database {
    /// Rebuild the shared library views after applying remote changes.
    pub async fn refresh_cloud_library(&self, connection: &str) -> Result<bool> {
        refresh(self, connection).await
    }

    pub async fn set_cloud_library_connection(&self, connection: Option<&str>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM cloud_library_session")
            .execute(&mut *tx)
            .await?;
        if let Some(connection) = connection {
            sqlx::query("INSERT INTO cloud_library_session VALUES (1,?)")
                .bind(connection)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("UPDATE files SET missing_since=1 WHERE cloud_connection_id IS NOT NULL AND missing_since IS NULL")
            .execute(&mut *tx).await?;
        sqlx::query("INSERT INTO cloud_library_pending VALUES(1) ON CONFLICT(slot) DO NOTHING")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

pub(super) async fn refresh(db: &Database, connection: &str) -> Result<bool> {
    let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cloud_library_pending) OR EXISTS(SELECT 1 FROM files f JOIN cloud_library_session s ON s.connection_id=f.cloud_connection_id WHERE f.cloud_connection_id=? AND f.missing_since>1)")
        .bind(connection).fetch_one(&db.pool).await?;
    if !pending {
        return Ok(false);
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let active: Option<String> =
        sqlx::query_scalar("SELECT connection_id FROM cloud_library_session WHERE slot=1")
            .fetch_optional(&mut *tx)
            .await?;
    if active.as_deref() != Some(connection) {
        return Ok(false);
    }
    let mut changed = false;
    let mut cursor = String::new();
    loop {
        let docs: Vec<(String,String)> = sqlx::query_as("SELECT id,document FROM cloud_tracks WHERE connection_id=? AND id>? ORDER BY id LIMIT 100")
            .bind(connection).bind(&cursor).fetch_all(&mut *tx).await?;
        if docs.is_empty() {
            break;
        }
        for (id, document) in docs {
            cursor = id;
            let track: Track = serde_json::from_str(&document)?;
            let path = format!("cloud-library://{connection}/{}", track.id);
            let existing: Option<(i64, Option<i64>)> = sqlx::query_as(
                "SELECT id,missing_since FROM files WHERE path=? AND cloud_connection_id=?",
            )
            .bind(&path)
            .bind(connection)
            .fetch_optional(&mut *tx)
            .await?;
            let local: Option<i64> = sqlx::query_scalar("SELECT f.id FROM files f WHERE f.cloud_connection_id IS NULL AND f.status='ok' AND f.missing_since IS NULL AND (EXISTS(SELECT 1 FROM file_cloud_links l WHERE l.file_id=f.id AND l.connection_id=? AND l.cloud_track_id=?) OR (f.hash=? AND f.size=?)) ORDER BY f.id LIMIT 1")
                .bind(connection).bind(&track.id).bind(&track.sha256).bind(track.size).fetch_optional(&mut *tx).await?;
            if let Some(local) = local.filter(|_| !track.deleted) {
                sqlx::query("INSERT INTO file_cloud_links(file_id,connection_id,cloud_track_id) VALUES (?,?,?) ON CONFLICT(file_id,connection_id) DO NOTHING")
                    .bind(local).bind(connection).bind(&track.id).execute(&mut *tx).await?;
            }
            if track.deleted || local.is_some() {
                if let Some((file, missing)) = existing {
                    if missing.is_none() {
                        sqlx::query("UPDATE files SET missing_since=1 WHERE id=?")
                            .bind(file)
                            .execute(&mut *tx)
                            .await?;
                        changed = true;
                    }
                    if let Some(local) = local {
                        transfer_state(&mut tx, file, local).await?;
                    }
                }
                continue;
            }
            let file = if let Some((file, missing)) = existing {
                if missing.is_some() {
                    sqlx::query("UPDATE files SET missing_since=NULL WHERE id=?")
                        .bind(file)
                        .execute(&mut *tx)
                        .await?;
                    changed = true;
                }
                file
            } else {
                let file = sqlx::query("INSERT INTO files(path,file_name,status,metadata_status,size,hash,duration_ms,format,last_validated,cloud_connection_id) VALUES (?,?,'ok','ok',?,?,?,?,unixepoch(),?)")
                    .bind(&path).bind(&track.file_name).bind(track.size).bind(&track.sha256).bind(track.duration_ms).bind(&track.format).bind(connection)
                    .execute(&mut *tx).await?.last_insert_rowid();
                sqlx::query("INSERT INTO file_cloud_links(file_id,connection_id,cloud_track_id) VALUES (?,?,?)")
                    .bind(file).bind(connection).bind(&track.id).execute(&mut *tx).await?;
                store::write_metadata(&mut tx, file, &track.metadata).await?;
                sqlx::query("INSERT INTO cloud_local_state(connection_id,file_id,metadata,revision) VALUES (?,?,?,?)")
                    .bind(connection).bind(file).bind(serde_json::to_string(&track.metadata)?).bind(track.revision).execute(&mut *tx).await?;
                changed = true;
                file
            };
            let missing: Vec<i64> = sqlx::query_scalar("SELECT f.id FROM files f JOIN file_cloud_links l ON l.file_id=f.id WHERE l.connection_id=? AND l.cloud_track_id=? AND f.cloud_connection_id IS NULL AND f.missing_since IS NOT NULL")
                .bind(connection).bind(&track.id).fetch_all(&mut *tx).await?;
            for missing in missing {
                transfer_state(&mut tx, missing, file).await?;
            }
            let current = sqlx::query("UPDATE files SET file_name=?,size=?,hash=?,duration_ms=?,format=? WHERE id=? AND (file_name IS NOT ? OR size IS NOT ? OR hash IS NOT ? OR duration_ms IS NOT ? OR format IS NOT ?)")
                .bind(&track.file_name).bind(track.size).bind(&track.sha256).bind(track.duration_ms).bind(&track.format).bind(file)
                .bind(&track.file_name).bind(track.size).bind(&track.sha256).bind(track.duration_ms).bind(&track.format).execute(&mut *tx).await?;
            changed |= current.rows_affected() > 0;
        }
    }
    changed |= store::reconcile_playlists(&mut tx, connection).await?;
    sqlx::query("DELETE FROM cloud_library_pending")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(changed)
}

async fn transfer_state(conn: &mut sqlx::SqliteConnection, from: i64, to: i64) -> Result<()> {
    sqlx::query("UPDATE playlist_tracks SET file_id=? WHERE file_id=?")
        .bind(to)
        .bind(from)
        .execute(&mut *conn)
        .await?;
    for table in ["listening_history", "counted_plays"] {
        sqlx::query(&format!("UPDATE {table} SET file_id=? WHERE file_id=?"))
            .bind(to)
            .bind(from)
            .execute(&mut *conn)
            .await?;
    }
    sqlx::query("INSERT INTO listening_time_daily(file_id,source_playlist_id,day,heard_us) SELECT ?,source_playlist_id,day,heard_us FROM listening_time_daily WHERE file_id=? ON CONFLICT(file_id,source_playlist_id,day) DO UPDATE SET heard_us=heard_us+excluded.heard_us")
                            .bind(to).bind(from).execute(&mut *conn).await?;
    sqlx::query("DELETE FROM listening_time_daily WHERE file_id=?")
        .bind(from)
        .execute(&mut *conn)
        .await?;
    sqlx::query("INSERT INTO media_info(file_id,plays,loved) SELECT ?,plays,loved FROM media_info WHERE file_id=? ON CONFLICT(file_id) DO UPDATE SET plays=plays+excluded.plays")
                            .bind(to).bind(from).execute(&mut *conn).await?;
    sqlx::query("UPDATE media_info SET plays=0 WHERE file_id=?")
        .bind(from)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
