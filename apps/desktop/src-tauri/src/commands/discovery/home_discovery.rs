use crate::AppState;
#[derive(serde::Serialize)]
pub struct HomeDiscovery {
    recent_ids: Vec<i64>,
    favorite_ids: Vec<i64>,
    album_ids: Vec<i64>,
    listening: super::listening::Listening,
}
#[tauri::command]
pub async fn get_home_discovery(
    state: tauri::State<'_, AppState>,
    since: Option<i64>,
    until: Option<i64>,
) -> Result<HomeDiscovery, String> {
    let mut tx = state.db.pool.begin().await.map_err(|e| e.to_string())?;
    let recent_ids = sqlx::query_scalar("SELECT id FROM files ORDER BY id DESC LIMIT 5")
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let favorite_ids = sqlx::query_scalar("SELECT f.id FROM files f JOIN media_info i ON i.file_id=f.id WHERE i.loved=1 ORDER BY RANDOM() LIMIT 5")
        .fetch_all(&mut *tx).await.map_err(|e| e.to_string())?;
    let album_ids = sqlx::query_scalar("WITH albums AS (
        SELECT f.id, a.value AS album, COALESCE((SELECT m.value FROM metadata_texts m
        WHERE m.file_id=f.id AND m.key IN ('albumArtist','artist') AND TRIM(m.value) <> ''
        ORDER BY CASE m.key WHEN 'albumArtist' THEN 0 ELSE 1 END, m.ord LIMIT 1), '') AS artist
        FROM files f JOIN metadata_texts a ON a.file_id=f.id AND a.key='album' WHERE TRIM(a.value) <> ''
    ) SELECT MIN(id) FROM albums GROUP BY album COLLATE NOCASE, artist COLLATE NOCASE ORDER BY RANDOM() LIMIT 5")
        .fetch_all(&mut *tx).await.map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    let listening = super::listening::read(&state.db.pool, since, until).await?;
    Ok(HomeDiscovery {
        recent_ids,
        favorite_ids,
        album_ids,
        listening,
    })
}
