use super::{Database, collections::Collection};
#[derive(Clone, Debug)]
pub struct MenuState {
    pub all_loved: bool,
    pub playlists: Vec<(i64, String)>,
    pub disk_paths: std::collections::BTreeMap<i64, String>,
    pub downloaded: std::collections::BTreeSet<i64>,
}
fn unique(ids: &[i64]) -> Vec<i64> {
    let mut result = Vec::new();
    for &id in ids {
        if id > 0 && !result.contains(&id) {
            result.push(id);
        }
    }
    result
}
impl Database {
    pub async fn library_disk_paths(
        &self,
        ids: &[i64],
    ) -> Result<std::collections::BTreeMap<i64, String>, sqlx::Error> {
        let mut paths = std::collections::BTreeMap::new();
        for id in unique(ids) {
            let row: Option<(Option<String>,Option<i64>)> = sqlx::query_as("SELECT CASE WHEN f.cloud_connection_id IS NULL THEN f.path ELSE d.path END,CASE WHEN f.cloud_connection_id IS NOT NULL THEN d.size END FROM files f LEFT JOIN file_cloud_links l ON l.file_id=f.id AND l.connection_id=f.cloud_connection_id LEFT JOIN cloud_downloads d ON d.connection_id=l.connection_id AND d.track_id=l.cloud_track_id AND d.sha256=f.hash WHERE f.id=? AND f.missing_since IS NULL AND (f.cloud_connection_id IS NULL OR f.cloud_connection_id=(SELECT connection_id FROM cloud_library_session WHERE slot=1))")
                .bind(id).fetch_optional(&self.pool).await?;
            if let Some((Some(path), size)) = row {
                if tokio::fs::metadata(&path).await.is_ok_and(|stat| {
                    stat.is_file() && size.is_none_or(|size| stat.len() == size as u64)
                }) {
                    paths.insert(id, path);
                }
            }
        }
        Ok(paths)
    }
    pub async fn media_menu_state(&self, ids: &[i64]) -> Result<MenuState, sqlx::Error> {
        let ids = unique(ids);
        let mut all_loved = !ids.is_empty();
        for &id in &ids {
            let loved = sqlx::query_scalar::<_, bool>(
                "SELECT COALESCE((SELECT loved FROM media_info WHERE file_id=?),0)",
            )
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
            all_loved &= loved;
        }
        let playlists =
            sqlx::query_as("SELECT id,name FROM library_playlists ORDER BY name COLLATE NOCASE,id")
                .fetch_all(&self.pool)
                .await?;
        let disk_paths = self.library_disk_paths(&ids).await?;
        let mut downloaded = std::collections::BTreeSet::new();
        for &id in &ids {
            if disk_paths.contains_key(&id) {
                let cloud: bool = sqlx::query_scalar(
                    "SELECT cloud_connection_id IS NOT NULL FROM files WHERE id=?",
                )
                .bind(id)
                .fetch_optional(&self.pool)
                .await?
                .unwrap_or(false);
                if cloud {
                    downloaded.insert(id);
                }
            }
        }
        Ok(MenuState {
            all_loved,
            playlists,
            disk_paths,
            downloaded,
        })
    }
    pub async fn favorite_selection(&self, ids: &[i64], loved: bool) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        for id in unique(ids) {
            sqlx::query("INSERT INTO media_info(file_id,loved) VALUES (?,?) ON CONFLICT(file_id) DO UPDATE SET loved=excluded.loved").bind(id).bind(loved).execute(&mut *tx).await?;
        }
        tx.commit().await
    }
    pub async fn add_selection_to_playlist(
        &self,
        playlist: i64,
        ids: &[i64],
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut ord = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(MAX(ord),-1)+1 FROM playlist_tracks WHERE playlist_id=?",
        )
        .bind(playlist)
        .fetch_one(&mut *tx)
        .await?;
        for id in unique(ids) {
            sqlx::query("INSERT INTO playlist_tracks(playlist_id,file_id,ord) VALUES (?,?,?)")
                .bind(playlist)
                .bind(id)
                .bind(ord)
                .execute(&mut *tx)
                .await?;
            ord += 1;
        }
        tx.commit().await
    }
    pub async fn remove_playlist_occurrences(
        &self,
        entries: &[(i64, i64, i64)],
    ) -> Result<(), sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        for &(playlist, ord, id) in entries {
            let result = sqlx::query(
                "DELETE FROM playlist_tracks WHERE playlist_id=? AND ord=? AND file_id=?",
            )
            .bind(playlist)
            .bind(ord)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
            if result.rows_affected() != 1 {
                return Err(sqlx::Error::RowNotFound);
            }
        }
        transaction.commit().await
    }
    pub async fn remove_playlist_occurrence(
        &self,
        playlist: i64,
        ord: i64,
        id: i64,
    ) -> Result<(), sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id=? AND ord=? AND file_id=?")
                .bind(playlist)
                .bind(ord)
                .bind(id)
                .execute(&self.pool)
                .await?;
        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }
    pub async fn album_for_track(&self, id: i64) -> Result<Option<Collection>, sqlx::Error> {
        let entry=sqlx::query_as::<_,(String,String)>("SELECT COALESCE((SELECT value FROM metadata_texts WHERE file_id=f.id AND key='album' AND TRIM(value)<>'' ORDER BY ord LIMIT 1),''),COALESCE((SELECT value FROM metadata_texts WHERE file_id=f.id AND key IN ('albumArtist','artist') AND TRIM(value)<>'' ORDER BY CASE key WHEN 'albumArtist' THEN 0 ELSE 1 END,ord LIMIT 1),'') FROM files f WHERE f.id=? AND f.missing_since IS NULL").bind(id).fetch_optional(&self.pool).await?;
        Ok(entry.map(|(name, artist)| Collection {
            name,
            artist,
            file_id: Some(id),
            playlist_id: None,
            track_count: 0,
        }))
    }
}
