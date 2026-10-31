use super::*;

pub(crate) async fn reconcile_playlists(
    conn: &mut SqliteConnection,
    connection: &str,
) -> Result<bool> {
    let mut changed = false;
    let mut cursor = String::new();
    loop {
        let documents: Vec<(String, String)> = sqlx::query_as("SELECT id,document FROM cloud_playlists WHERE connection_id=? AND id>? ORDER BY id LIMIT 100")
            .bind(connection).bind(&cursor).fetch_all(&mut *conn).await?;
        if documents.is_empty() {
            break;
        }
        for (remote_id, document) in documents {
            cursor = remote_id;
            let playlist: Playlist = serde_json::from_str(&document)?;
            let link: Option<(i64, String, i64)> = sqlx::query_as("SELECT playlist_id,snapshot,revision FROM cloud_playlist_links WHERE connection_id=? AND cloud_id=?")
                .bind(connection).bind(&playlist.id).fetch_optional(&mut *conn).await?;
            if let Some((id, baseline, revision)) = &link {
                if *revision > playlist.revision {
                    continue;
                }
                let name: String = sqlx::query_scalar("SELECT name FROM playlists WHERE id=?")
                    .bind(id)
                    .fetch_one(&mut *conn)
                    .await?;
                let members: Vec<Option<String>> = sqlx::query_scalar("SELECT l.cloud_track_id FROM playlist_tracks pt LEFT JOIN file_cloud_links l ON l.file_id=pt.file_id AND l.connection_id=? WHERE pt.playlist_id=? ORDER BY pt.ord")
                    .bind(connection).bind(id).fetch_all(&mut *conn).await?;
                if members.iter().any(Option::is_none)
                    || serde_json::to_string(&(
                        name,
                        members.into_iter().flatten().collect::<Vec<_>>(),
                    ))? != *baseline
                {
                    continue;
                }
                if playlist.deleted {
                    sqlx::query(
                        "DELETE FROM cloud_playlist_links WHERE connection_id=? AND playlist_id=?",
                    )
                    .bind(connection)
                    .bind(id)
                    .execute(&mut *conn)
                    .await?;
                    sqlx::query("DELETE FROM playlists WHERE id=?")
                        .bind(id)
                        .execute(&mut *conn)
                        .await?;
                    changed = true;
                    continue;
                }
                if *revision == playlist.revision {
                    continue;
                }
            } else {
                if playlist.deleted {
                    continue;
                }
                let deleting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cloud_outbox WHERE connection_id=? AND json_extract(payload,'$.request.edit.kind')='delete_playlist' AND json_extract(payload,'$.request.edit.playlist_id')=?)")
                    .bind(connection).bind(&playlist.id).fetch_one(&mut *conn).await?;
                if deleting {
                    continue;
                }
            }
            let mut members = Vec::with_capacity(playlist.track_ids.len());
            for remote in &playlist.track_ids {
                let local: Option<i64> = sqlx::query_scalar("SELECT f.id FROM files f JOIN file_cloud_links l ON l.file_id=f.id WHERE l.connection_id=? AND l.cloud_track_id=? AND f.missing_since IS NULL AND (f.cloud_connection_id IS NULL OR f.cloud_connection_id=?) ORDER BY f.cloud_connection_id IS NOT NULL,f.id LIMIT 1")
                    .bind(connection).bind(remote).bind(connection).fetch_optional(&mut *conn).await?;
                let Some(local) = local else {
                    break;
                };
                members.push(local);
            }
            if members.len() != playlist.track_ids.len() {
                continue;
            }
            let id = if let Some((id, _, _)) = link {
                sqlx::query("UPDATE playlists SET name=? WHERE id=?")
                    .bind(&playlist.name)
                    .bind(id)
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id=?")
                    .bind(id)
                    .execute(&mut *conn)
                    .await?;
                id
            } else {
                sqlx::query("INSERT INTO playlists(name,cloud_connection_id) VALUES (?,?)")
                    .bind(&playlist.name)
                    .bind(connection)
                    .execute(&mut *conn)
                    .await?
                    .last_insert_rowid()
            };
            for (ord, file) in members.into_iter().enumerate() {
                sqlx::query("INSERT INTO playlist_tracks(playlist_id,file_id,ord) VALUES (?,?,?)")
                    .bind(id)
                    .bind(file)
                    .bind(ord as i64)
                    .execute(&mut *conn)
                    .await?;
            }
            sqlx::query("INSERT INTO cloud_playlist_links(connection_id,playlist_id,cloud_id,snapshot,revision) VALUES (?,?,?,?,?) ON CONFLICT(connection_id,playlist_id) DO UPDATE SET snapshot=excluded.snapshot,revision=excluded.revision")
                .bind(connection).bind(id).bind(&playlist.id).bind(serde_json::to_string(&(playlist.name, playlist.track_ids))?).bind(playlist.revision).execute(&mut *conn).await?;
            changed = true;
        }
    }
    Ok(changed)
}
