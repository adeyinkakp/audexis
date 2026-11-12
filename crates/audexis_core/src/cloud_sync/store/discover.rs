use super::*;

pub async fn discover_with_uploads(
    pool: &SqlitePool,
    connection: &str,
    uploads: bool,
) -> Result<()> {
    crate::database::smart_playlists::refresh(pool).await?;
    let mut cursor = 0_i64;
    loop {
        let rows=sqlx::query("SELECT f.id,f.path,f.file_name,f.hash,f.size,f.added_at,l.cloud_track_id FROM files f LEFT JOIN file_cloud_links l ON l.file_id=f.id AND l.connection_id=? WHERE f.id>? AND (f.cloud_connection_id IS NULL OR f.cloud_connection_id=?) AND f.status='ok' AND f.metadata_status='ok' AND f.missing_since IS NULL ORDER BY f.id LIMIT 200").bind(connection).bind(cursor).bind(connection).fetch_all(pool).await?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id: i64 = row.get("id");
            cursor = id;
            let mut conn = pool.acquire().await?;
            let metadata = local_metadata(&mut conn, id).await?;
            let linked: Option<String> = row.get("cloud_track_id");
            drop(conn);
            if let Some(cloud_id) = linked {
                let document: Option<String> = sqlx::query_scalar(
                    "SELECT document FROM cloud_tracks WHERE connection_id=? AND id=?",
                )
                .bind(connection)
                .bind(&cloud_id)
                .fetch_optional(pool)
                .await?;
                let Some(document) = document else { continue };
                let track: Track = serde_json::from_str(&document)?;
                if track.deleted {
                    continue;
                }
                let baseline: Option<String> = sqlx::query_scalar(
                    "SELECT metadata FROM cloud_local_state WHERE connection_id=? AND file_id=?",
                )
                .bind(connection)
                .bind(id)
                .fetch_optional(pool)
                .await?;
                let encoded = serde_json::to_string(&metadata)?;
                if baseline.as_deref() != Some(&encoded) {
                    let job = Job::Mutate {
                        file_id: Some(id),
                        playlist_id: None,
                        request: Mutation {
                            operation_id: uuid::Uuid::new_v4().to_string(),
                            base_revision: sqlx::query_scalar("SELECT revision FROM cloud_local_state WHERE connection_id=? AND file_id=?").bind(connection).bind(id).fetch_optional(pool).await?.unwrap_or(track.revision),
                            edit: Edit::Track {
                                track_id: cloud_id,
                                metadata,
                            },
                        },
                    };
                    enqueue(pool, connection, &format!("track:{id}"), &job).await?;
                }
            } else {
                if !uploads || row.get::<String, _>("path").starts_with("cloud-library://") {
                    continue;
                }
                let hash: Option<String> = row.get("hash");
                let Some(hash) = hash.filter(|h| h.len() == 64) else {
                    continue;
                };
                let size: i64 = row.get("size");
                if size <= 0 {
                    continue;
                }
                let request = UploadRequest {
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    sha256: hash,
                    size,
                    file_name: row.get("file_name"),
                    added_at: row.get("added_at"),
                    metadata,
                };
                enqueue(
                    pool,
                    connection,
                    &format!("upload:{id}"),
                    &Job::Upload {
                        file_id: id,
                        request,
                        upload_id: None,
                        uploaded: false,
                    },
                )
                .await?;
            }
        }
    }
    let playlists: Vec<(i64, String, Option<String>)> = sqlx::query_as("SELECT id,name,smart_rules FROM playlists p WHERE cloud_enabled=1 AND (cloud_connection_id IS NULL OR cloud_connection_id=?1) AND (NOT EXISTS(SELECT 1 FROM cloud_playlist_links l WHERE l.playlist_id=p.id) OR EXISTS(SELECT 1 FROM cloud_playlist_links l WHERE l.playlist_id=p.id AND l.connection_id=?1)) ORDER BY id")
        .bind(connection)
        .fetch_all(pool)
        .await?;
    for (id, name, encoded) in playlists {
        let smart_rules: Option<SmartPlaylist> =
            encoded.map(|s| serde_json::from_str(&s)).transpose()?;
        let members:Vec<(i64,Option<String>)>=sqlx::query_as("SELECT pt.file_id,l.cloud_track_id FROM playlist_tracks pt LEFT JOIN file_cloud_links l ON l.file_id=pt.file_id AND l.connection_id=? WHERE pt.playlist_id=? ORDER BY pt.ord").bind(connection).bind(id).fetch_all(pool).await?;
        if members.iter().any(|(_, remote)| remote.is_none()) {
            continue;
        }
        let ids: Vec<String> = members.into_iter().filter_map(|(_, id)| id).collect();
        let link:Option<(String,String,i64)>=sqlx::query_as("SELECT cloud_id,snapshot,revision FROM cloud_playlist_links WHERE connection_id=? AND playlist_id=?").bind(connection).bind(id).fetch_optional(pool).await?;
        if link
            .as_ref()
            .is_some_and(|(_, saved, _)| playlist_unchanged(saved, &name, &ids, &smart_rules))
        {
            continue;
        }
        let cloud_id = link
            .as_ref()
            .map(|(id, _, _)| id.clone())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let doc: Option<String> = sqlx::query_scalar(
            "SELECT document FROM cloud_playlists WHERE connection_id=? AND id=?",
        )
        .bind(connection)
        .bind(&cloud_id)
        .fetch_optional(pool)
        .await?;
        let remote = doc
            .map(|d| serde_json::from_str::<Playlist>(&d))
            .transpose()?;
        if remote.as_ref().is_some_and(|p| p.deleted) {
            continue;
        }
        let revision = link
            .as_ref()
            .map(|(_, _, rev)| *rev)
            .or_else(|| remote.as_ref().map(|p| p.revision))
            .unwrap_or(0);
        enqueue(
            pool,
            connection,
            &format!("playlist:{id}"),
            &Job::Mutate {
                file_id: None,
                playlist_id: Some(id),
                request: Mutation {
                    operation_id: uuid::Uuid::new_v4().to_string(),
                    base_revision: revision,
                    edit: Edit::Playlist {
                        playlist_id: cloud_id,
                        name,
                        track_ids: ids,
                        smart_rules,
                    },
                },
            },
        )
        .await?;
    }
    Ok(())
}
