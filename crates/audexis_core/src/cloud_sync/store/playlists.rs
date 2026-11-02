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
            sqlx::query("UPDATE cloud_outbox SET connection_id=?1 WHERE connection_id<>?1 AND connection_id IN (SELECT old.id FROM connections old JOIN connections current ON current.id=?1 WHERE old.server_user_id=current.server_user_id) AND json_extract(payload,'$.request.edit.playlist_id')=?2 AND NOT EXISTS(SELECT 1 FROM cloud_outbox other WHERE other.connection_id=?1 AND other.entity_key=cloud_outbox.entity_key)")
                .bind(connection).bind(&playlist.id).execute(&mut *conn).await?;

            let mut link: Option<(i64, String, i64)> = sqlx::query_as("SELECT playlist_id,snapshot,revision FROM cloud_playlist_links WHERE connection_id=? AND cloud_id=?")
                .bind(connection).bind(&playlist.id).fetch_optional(&mut *conn).await?;

            if link.is_none() {
                let previous: Option<(i64,String,i64,String)> = sqlx::query_as("SELECT l.playlist_id,l.snapshot,l.revision,l.connection_id FROM cloud_playlist_links l JOIN playlists p ON p.id=l.playlist_id JOIN connections old ON old.id=l.connection_id JOIN connections current ON current.id=? WHERE l.cloud_id=? AND l.connection_id<>current.id AND old.server_user_id=current.server_user_id AND p.cloud_enabled=1 ORDER BY l.revision DESC LIMIT 1")
                    .bind(connection).bind(&playlist.id).fetch_optional(&mut *conn).await?;

                if let Some((id, baseline, revision, previous)) = previous {
                    let entries: Vec<(i64,String)> = sqlx::query_as("SELECT pt.ord,l.cloud_track_id FROM playlist_tracks pt JOIN file_cloud_links l ON l.file_id=pt.file_id AND l.connection_id=? WHERE pt.playlist_id=?")
                        .bind(&previous).bind(id).fetch_all(&mut *conn).await?;

                    for (ord, remote) in entries {
                        let file: Option<i64> = sqlx::query_scalar("SELECT f.id FROM file_cloud_links l JOIN files f ON f.id=l.file_id WHERE l.connection_id=? AND l.cloud_track_id=? AND f.missing_since IS NULL AND (f.cloud_connection_id IS NULL OR f.cloud_connection_id=?) ORDER BY f.cloud_connection_id IS NOT NULL,f.id LIMIT 1")
                            .bind(connection).bind(remote).bind(connection).fetch_optional(&mut *conn).await?;

                        if let Some(file) = file {
                            sqlx::query("UPDATE playlist_tracks SET file_id=? WHERE playlist_id=? AND ord=?").bind(file).bind(id).bind(ord).execute(&mut *conn).await?;
                        }
                    }
                    sqlx::query("INSERT INTO cloud_playlist_links(connection_id,playlist_id,cloud_id,snapshot,revision) VALUES (?,?,?,?,?)")
                        .bind(connection).bind(id).bind(&playlist.id).bind(&baseline).bind(revision).execute(&mut *conn).await?;

                    sqlx::query("UPDATE cloud_outbox SET connection_id=? WHERE connection_id=? AND json_extract(payload,'$.playlist_id')=? AND NOT EXISTS(SELECT 1 FROM cloud_outbox other WHERE other.connection_id=? AND other.entity_key=cloud_outbox.entity_key)")
                        .bind(connection).bind(&previous).bind(id).bind(connection).execute(&mut *conn).await?;

                    sqlx::query(
                        "DELETE FROM cloud_playlist_links WHERE connection_id=? AND playlist_id=?",
                    )
                    .bind(&previous)
                    .bind(id)
                    .execute(&mut *conn)
                    .await?;
                    sqlx::query("UPDATE playlists SET cloud_connection_id=NULL WHERE id=?")
                        .bind(id)
                        .execute(&mut *conn)
                        .await?;
                    link = Some((id, baseline, revision));
                    changed = true;
                }
            }
            if let Some((id, baseline, revision)) = &link {
                if *revision > playlist.revision {
                    continue;
                }
                let (name, rules): (String, Option<String>) =
                    sqlx::query_as("SELECT name,smart_rules FROM playlists WHERE id=?")
                        .bind(id)
                        .fetch_one(&mut *conn)
                        .await?;
                let members: Vec<Option<String>> = sqlx::query_scalar("SELECT l.cloud_track_id FROM playlist_tracks pt LEFT JOIN file_cloud_links l ON l.file_id=pt.file_id AND l.connection_id=? WHERE pt.playlist_id=? ORDER BY pt.ord")
                    .bind(connection).bind(id).fetch_all(&mut *conn).await?;
                let rules = rules
                    .map(|s| serde_json::from_str::<SmartPlaylist>(&s))
                    .transpose()?;
                if (rules.is_none() && members.iter().any(Option::is_none))
                    || !playlist_unchanged(
                        baseline,
                        &name,
                        &members.into_iter().flatten().collect::<Vec<_>>(),
                        &rules,
                    )
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
                sqlx::query("UPDATE playlists SET name=?,smart_rules=?,cloud_enabled=1 WHERE id=?")
                    .bind(&playlist.name)
                    .bind(
                        playlist
                            .smart_rules
                            .as_ref()
                            .map(serde_json::to_string)
                            .transpose()?,
                    )
                    .bind(id)
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id=?")
                    .bind(id)
                    .execute(&mut *conn)
                    .await?;
                id
            } else {
                sqlx::query("INSERT INTO playlists(name,smart_rules,cloud_enabled) VALUES (?,?,1)")
                    .bind(&playlist.name)
                    .bind(
                        playlist
                            .smart_rules
                            .as_ref()
                            .map(serde_json::to_string)
                            .transpose()?,
                    )
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
                .bind(connection).bind(id).bind(&playlist.id).bind(playlist_snapshot(&playlist.name, &playlist.track_ids, &playlist.smart_rules)?).bind(playlist.revision).execute(&mut *conn).await?;
            changed = true;
        }
    }
    Ok(changed)
}
