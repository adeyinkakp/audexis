use crate::{
    AppState, db,
    error::{Error, Result},
};
use audexis_core::cloud_sync::protocol::{ChangeValue, Playlist, SmartPlaylist, Track};
use sqlx::{Postgres, Transaction};

pub async fn members(
    tx: &mut Transaction<'_, Postgres>,
    user: &str,
    rules: &SmartPlaylist,
) -> Result<Vec<String>> {
    rules.validate().map_err(Error::bad)?;
    let docs: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT document FROM cloud_tracks WHERE user_id=$1 AND NOT deleted ORDER BY id",
    )
    .bind(user)
    .fetch_all(&mut **tx)
    .await?;
    let tracks: Vec<Track> = docs
        .into_iter()
        .map(serde_json::from_value)
        .collect::<std::result::Result<_, _>>()?;
    Ok(rules.select(&tracks, audexis_core::database::smart_playlists::now()))
}

pub async fn refresh(state: &AppState, user: &str) -> Result<()> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cloud_playlists WHERE user_id=$1 AND NOT deleted AND document->'smart_rules' IS NOT NULL AND document->'smart_rules'<>'null'::jsonb AND COALESCE(document->'smart_rules'->>'live_updating','true')='true')")
        .bind(user).fetch_one(&state.db).await?;
    if !exists {
        return Ok(());
    }
    let mut tx = state.db.begin().await?;
    db::lock_account(&mut tx, user).await?;
    let docs: Vec<serde_json::Value> = sqlx::query_scalar("SELECT document FROM cloud_playlists WHERE user_id=$1 AND NOT deleted AND document->'smart_rules' IS NOT NULL AND COALESCE(document->'smart_rules'->>'live_updating','true')='true' FOR UPDATE")
        .bind(user).fetch_all(&mut *tx).await?;
    let tracks: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT document FROM cloud_tracks WHERE user_id=$1 AND NOT deleted ORDER BY id",
    )
    .bind(user)
    .fetch_all(&mut *tx)
    .await?;
    let tracks: Vec<Track> = tracks
        .into_iter()
        .map(serde_json::from_value)
        .collect::<std::result::Result<_, _>>()?;
    for doc in docs {
        let mut playlist: Playlist = serde_json::from_value(doc)?;
        let Some(rules) = &playlist.smart_rules else {
            continue;
        };
        if !rules.live_updating {
            continue;
        }
        rules.validate().map_err(Error::bad)?;
        let selected = rules.select(&tracks, audexis_core::database::smart_playlists::now());
        if selected == playlist.track_ids {
            continue;
        }
        playlist.track_ids = selected;
        playlist.revision += 1;
        sqlx::query("UPDATE cloud_playlists SET document=$1 WHERE id=$2 AND user_id=$3")
            .bind(serde_json::to_value(&playlist)?)
            .bind(&playlist.id)
            .bind(user)
            .execute(&mut *tx)
            .await?;
        db::change(&mut tx, user, &ChangeValue::Playlist(playlist)).await?;
    }
    tx.commit().await?;
    Ok(())
}
