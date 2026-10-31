use super::*;

pub(super) async fn update_job(
    context: &CloudSyncContext,
    operation: &str,
    job: &Job,
    delay: i64,
) -> Result<()> {
    sqlx::query("UPDATE cloud_outbox SET payload=?,retry_at=unixepoch()+?,error=NULL WHERE connection_id=? AND operation_id=?").bind(serde_json::to_string(job)?).bind(delay).bind(&context.connection_id).bind(operation).execute(&context.db.pool).await?;
    Ok(())
}
pub(super) async fn save_upload_state(
    context: &CloudSyncContext,
    operation: &str,
    id: &str,
    uploaded: bool,
) -> Result<()> {
    sqlx::query("UPDATE cloud_outbox SET payload=json_set(payload,'$.upload_id',?,'$.uploaded',json(?)),error=NULL WHERE connection_id=? AND operation_id=?").bind(id).bind(if uploaded {"true"} else {"false"}).bind(&context.connection_id).bind(operation).execute(&context.db.pool).await?;
    Ok(())
}

pub(super) async fn process(
    context: &CloudSyncContext,
    client: &CloudClient,
    pending: &Pending,
    progress: &tokio::sync::watch::Sender<super::super::CloudSyncProgress>,
) -> Result<()> {
    let mut job: Job = serde_json::from_str(&pending.payload)?;
    match &mut job {
        Job::Upload {
            file_id,
            request,
            upload_id,
            uploaded,
        } => {
            let local:Option<(String,Option<String>,i64)>=sqlx::query_as("SELECT path,hash,size FROM files WHERE id=? AND status='ok' AND missing_since IS NULL").bind(*file_id).fetch_optional(&context.db.pool).await?;
            let Some((path, hash, size)) = local else {
                return complete(context, &pending.operation_id).await;
            };
            if hash.as_deref() != Some(&request.sha256) || size != request.size {
                if let Some(id) = upload_id {
                    let _ = client.cancel_upload(id).await;
                }
                return complete(context, &pending.operation_id).await;
            }
            progress.send_modify(|p| {
                p.phase = super::super::SyncPhase::Processing;
                p.file_name = Some(request.file_name.clone());
                p.bytes_sent = 0;
                p.bytes_total = 0;
            });
            let mut upload: Upload = if let Some(id) = upload_id.as_ref() {
                match client.get(&format!("/uploads/{id}")).await {
                    Err(error) if error.status == Some(404) => {
                        *uploaded = false;
                        client.post("/uploads", request).await?
                    }
                    result => result?,
                }
            } else {
                client.post("/uploads", request).await?
            };
            *upload_id = Some(upload.id.clone());
            save_upload_state(context, &pending.operation_id, &upload.id, *uploaded).await?;
            if upload.state == "created" {
                if !*uploaded {
                    let url = upload
                        .upload_url
                        .as_deref()
                        .ok_or_else(|| SyncError::local("Upload URL missing"))?;
                    progress.send_modify(|p| {
                        p.phase = super::super::SyncPhase::Uploading;
                        p.bytes_total = size as u64;
                        p.bytes_sent = 0;
                    });
                    let reporter = progress.clone();
                    client
                        .upload_with_progress(url, Path::new(&path), size, move |sent| {
                            reporter.send_modify(|p| p.bytes_sent = sent.min(p.bytes_total));
                        })
                        .await?;
                    *uploaded = true;
                    save_upload_state(context, &pending.operation_id, &upload.id, true).await?;
                }
                progress.send_modify(|p| {
                    p.phase = super::super::SyncPhase::Processing;
                    p.bytes_sent = p.bytes_total;
                });
                upload = client
                    .post(
                        &format!("/uploads/{}/complete", upload.id),
                        &serde_json::json!({}),
                    )
                    .await?;
            }
            if upload.state == "ready" {
                let track = upload
                    .track
                    .ok_or_else(|| SyncError::local("Processed track is missing"))?;
                let mut tx = context.db.pool.begin_with("BEGIN IMMEDIATE").await?;
                sqlx::query("INSERT INTO file_cloud_links(file_id,connection_id,cloud_track_id) VALUES (?,?,?) ON CONFLICT(file_id,connection_id) DO UPDATE SET cloud_track_id=excluded.cloud_track_id").bind(*file_id).bind(&context.connection_id).bind(&track.id).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO cloud_local_state(connection_id,file_id,metadata,revision) VALUES (?,?,?,?) ON CONFLICT(connection_id,file_id) DO UPDATE SET metadata=excluded.metadata,revision=excluded.revision").bind(&context.connection_id).bind(*file_id).bind(serde_json::to_string(&request.metadata)?).bind(track.revision).execute(&mut *tx).await?;
                tx.commit().await?;
                store::apply(
                    &context.db.pool,
                    &context.connection_id,
                    &[Change {
                        cursor: 0,
                        value: ChangeValue::Track(track),
                    }],
                    None,
                )
                .await?;
                complete(context, &pending.operation_id).await?;
                context.files.publish_changed(Vec::new());
            } else if matches!(upload.state.as_str(), "failed" | "expired" | "cancelled") {
                let error = upload.error.map(|message| SyncError {
                    status: Some(upload.error_status.unwrap_or(502)),
                    message,
                    retry_after: 0,
                });
                request.operation_id = uuid::Uuid::new_v4().to_string();
                *upload_id = None;
                *uploaded = false;
                update_job(context, &pending.operation_id, &job, 3).await?;
                if let Some(error) = error {
                    return Err(error);
                }
            } else {
                update_job(context, &pending.operation_id, &job, 2).await?;
            }
        }
        Job::Mutate {
            file_id,
            playlist_id,
            request,
        } => {
            progress.send_modify(|p| {
                p.phase = super::super::SyncPhase::Updating;
                p.file_name = None;
                p.bytes_sent = 0;
                p.bytes_total = 0;
            });
            let result: MutationResult = match client.post("/sync/mutations", request).await {
                Err(mut error) if error.status == Some(404) => {
                    error.status = Some(409);
                    error.message="This item was removed from the cloud. Discard its queued change to continue.".into();
                    return Err(error);
                }
                result => result?,
            };
            let mut tx = context.db.pool.begin_with("BEGIN IMMEDIATE").await?;
            if let (Some(file_id), ChangeValue::Track(track)) = (file_id, &result.value) {
                sqlx::query("INSERT INTO cloud_local_state(connection_id,file_id,metadata,revision) VALUES (?,?,?,?) ON CONFLICT(connection_id,file_id) DO UPDATE SET metadata=excluded.metadata,revision=excluded.revision").bind(&context.connection_id).bind(*file_id).bind(serde_json::to_string(&track.metadata)?).bind(track.revision).execute(&mut *tx).await?;
            }
            if let (Some(playlist_id), ChangeValue::Playlist(playlist)) =
                (playlist_id, &result.value)
            {
                let exists: bool =
                    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM playlists WHERE id=?)")
                        .bind(*playlist_id)
                        .fetch_one(&mut *tx)
                        .await?;
                if !exists {
                    tx.commit().await?;
                    store::enqueue(
                        &context.db.pool,
                        &context.connection_id,
                        &format!("playlist-delete:{}", playlist.id),
                        &Job::Mutate {
                            file_id: None,
                            playlist_id: None,
                            request: Mutation {
                                operation_id: uuid::Uuid::new_v4().to_string(),
                                base_revision: playlist.revision,
                                edit: Edit::DeletePlaylist {
                                    playlist_id: playlist.id.clone(),
                                },
                            },
                        },
                    )
                    .await?;
                    store::apply(
                        &context.db.pool,
                        &context.connection_id,
                        &[Change {
                            cursor: 0,
                            value: result.value.clone(),
                        }],
                        None,
                    )
                    .await?;
                    return complete(context, &pending.operation_id).await;
                }

                let snapshot =
                    serde_json::to_string(&(playlist.name.clone(), playlist.track_ids.clone()))?;
                sqlx::query("INSERT INTO cloud_playlist_links(connection_id,playlist_id,cloud_id,snapshot,revision) VALUES (?,?,?,?,?) ON CONFLICT(connection_id,playlist_id) DO UPDATE SET cloud_id=excluded.cloud_id,snapshot=excluded.snapshot,revision=excluded.revision").bind(&context.connection_id).bind(*playlist_id).bind(&playlist.id).bind(snapshot).bind(playlist.revision).execute(&mut *tx).await?;
            }
            tx.commit().await?;
            store::apply(
                &context.db.pool,
                &context.connection_id,
                &[Change {
                    cursor: 0,
                    value: result.value,
                }],
                None,
            )
            .await?;
            complete(context, &pending.operation_id).await?;
            context.files.publish_changed(Vec::new());
        }
    }
    Ok(())
}
pub(super) async fn complete(context: &CloudSyncContext, operation: &str) -> Result<()> {
    sqlx::query("DELETE FROM cloud_outbox WHERE connection_id=? AND operation_id=?")
        .bind(&context.connection_id)
        .bind(operation)
        .execute(&context.db.pool)
        .await?;
    Ok(())
}
