use super::*;

pub async fn mutate(
    State(state): State<Arc<AppState>>,
    Extension(account): Extension<Account>,
    Json(request): Json<Mutation>,
) -> Result<Json<MutationResult>> {
    check_operation(&request.operation_id)?;
    if request.base_revision < 0 {
        return Err(Error::bad("Invalid revision"));
    }
    let request_hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&request)?));
    let mut tx = state.db.begin().await?;
    db::lock_account(&mut tx, &account.user_id).await?;
    let old: Option<(String, serde_json::Value)> = sqlx::query_as(
        "SELECT request_hash,result FROM cloud_mutations WHERE user_id=$1 AND operation_id=$2",
    )
    .bind(&account.user_id)
    .bind(&request.operation_id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((hash, result)) = old {
        if hash != request_hash {
            return Err(Error(
                StatusCode::CONFLICT,
                "Operation ID already used".into(),
            ));
        }
        return Ok(Json(serde_json::from_value(result)?));
    }
    let value = match &request.edit {
        Edit::Track { track_id, metadata } => {
            check_metadata(metadata)?;
            let mut track = db::track(&mut tx, &account.user_id, track_id).await?;
            revision(track.revision, request.base_revision)?;
            track.revision += 1;
            track.metadata = metadata.clone();
            sqlx::query("UPDATE cloud_tracks SET document=$1 WHERE id=$2 AND user_id=$3")
                .bind(serde_json::to_value(&track)?)
                .bind(track_id)
                .bind(&account.user_id)
                .execute(&mut *tx)
                .await?;
            ChangeValue::Track(track)
        }
        Edit::DeleteTrack { track_id } => {
            let mut track = db::track(&mut tx, &account.user_id, track_id).await?;
            revision(track.revision, request.base_revision)?;
            track.revision += 1;
            track.deleted = true;
            sqlx::query(
                "UPDATE cloud_tracks SET deleted=TRUE,document=$1 WHERE id=$2 AND user_id=$3",
            )
            .bind(serde_json::to_value(&track)?)
            .bind(track_id)
            .bind(&account.user_id)
            .execute(&mut *tx)
            .await?;
            let playlists: Vec<serde_json::Value> = sqlx::query_scalar(
                "SELECT document FROM cloud_playlists WHERE user_id=$1 AND NOT deleted FOR UPDATE",
            )
            .bind(&account.user_id)
            .fetch_all(&mut *tx)
            .await?;
            for document in playlists {
                let mut playlist: Playlist = serde_json::from_value(document)?;
                let before = playlist.track_ids.len();
                playlist.track_ids.retain(|id| id != track_id);
                if before != playlist.track_ids.len() {
                    playlist.revision += 1;
                    sqlx::query("UPDATE cloud_playlists SET document=$1 WHERE id=$2")
                        .bind(serde_json::to_value(&playlist)?)
                        .bind(&playlist.id)
                        .execute(&mut *tx)
                        .await?;
                    db::change(&mut tx, &account.user_id, &ChangeValue::Playlist(playlist)).await?
                }
            }
            ChangeValue::Track(track)
        }
        Edit::Playlist {
            playlist_id,
            name,
            track_ids,
        } => {
            if uuid::Uuid::parse_str(playlist_id).is_err()
                || name.trim().is_empty()
                || name.len() > 512
                || track_ids.len() > 10000
            {
                return Err(Error::bad("Invalid playlist"));
            }
            let existing = db::playlist(&mut tx, &account.user_id, playlist_id).await?;
            revision(
                existing.as_ref().map(|p| p.revision).unwrap_or(0),
                request.base_revision,
            )?;
            let ids: Vec<String> = sqlx::query_scalar(
                "SELECT id FROM cloud_tracks WHERE user_id=$1 AND id=ANY($2) AND NOT deleted",
            )
            .bind(&account.user_id)
            .bind(track_ids)
            .fetch_all(&mut *tx)
            .await?;
            if track_ids.iter().any(|id| !ids.contains(id)) {
                return Err(Error::bad("Playlist contains unavailable tracks"));
            }
            let playlist = Playlist {
                id: playlist_id.clone(),
                revision: request.base_revision + 1,
                name: name.clone(),
                track_ids: track_ids.clone(),
                deleted: false,
            };
            let updated = sqlx::query("INSERT INTO cloud_playlists(id,user_id,document) VALUES ($1,$2,$3) ON CONFLICT(id) DO UPDATE SET document=excluded.document WHERE cloud_playlists.user_id=excluded.user_id AND NOT cloud_playlists.deleted").bind(playlist_id).bind(&account.user_id).bind(serde_json::to_value(&playlist)?).execute(&mut *tx).await? ;
            if updated.rows_affected() != 1 {
                return Err(Error(
                    StatusCode::CONFLICT,
                    "Playlist ID is unavailable".into(),
                ));
            }
            ChangeValue::Playlist(playlist)
        }
        Edit::DeletePlaylist { playlist_id } => {
            let mut playlist = db::playlist(&mut tx, &account.user_id, playlist_id)
                .await?
                .ok_or_else(|| Error(StatusCode::NOT_FOUND, "Playlist not found".into()))?;
            revision(playlist.revision, request.base_revision)?;
            playlist.deleted = true;
            playlist.revision += 1;
            sqlx::query("UPDATE cloud_playlists SET deleted=TRUE,document=$1 WHERE id=$2")
                .bind(serde_json::to_value(&playlist)?)
                .bind(playlist_id)
                .execute(&mut *tx)
                .await?;
            ChangeValue::Playlist(playlist)
        }
    };
    db::change(&mut tx, &account.user_id, &value).await?;
    let result = MutationResult { value };
    sqlx::query("INSERT INTO cloud_mutations(user_id,operation_id,request_hash,result) VALUES ($1,$2,$3,$4)").bind(&account.user_id).bind(&request.operation_id).bind(request_hash).bind(serde_json::to_value(&result)?).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(result))
}
pub(super) fn revision(current: i64, expected: i64) -> Result<()> {
    if current != expected {
        Err(Error(
            StatusCode::CONFLICT,
            "This item changed on another device. Pull changes and resolve the conflict.".into(),
        ))
    } else {
        Ok(())
    }
}
