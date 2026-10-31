use super::{CloudSyncContext, client::Result, protocol::Track};

impl CloudSyncContext {
    pub(super) async fn record_download(&self, track: &Track) -> Result<()> {
        let path = self.cache_path(track);
        sqlx::query("INSERT INTO cloud_downloads(connection_id,track_id,path,sha256,size) VALUES (?,?,?,?,?) ON CONFLICT(connection_id,track_id) DO UPDATE SET path=excluded.path,sha256=excluded.sha256,size=excluded.size")
            .bind(&self.connection_id).bind(&track.id).bind(path.to_string_lossy().as_ref()).bind(&track.sha256).bind(track.size).execute(&self.db.pool).await?;
        Ok(())
    }
    pub async fn reconcile_downloads(&self) -> Result<()> {
        let scanned: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM cloud_downloads_scanned WHERE connection_id=?)",
        )
        .bind(&self.connection_id)
        .fetch_one(&self.db.pool)
        .await?;
        let mut changed = false;
        if !scanned {
            let mut cursor = String::new();
            loop {
                let docs: Vec<(String,String)> = sqlx::query_as("SELECT id,document FROM cloud_tracks WHERE connection_id=? AND id>? AND json_extract(document,'$.deleted')=0 ORDER BY id LIMIT 100")
                    .bind(&self.connection_id).bind(&cursor).fetch_all(&self.db.pool).await?;
                if docs.is_empty() {
                    break;
                }
                for (id, doc) in docs {
                    cursor = id;
                    let track: Track = serde_json::from_str(&doc)?;
                    let path = self.cache_path(&track);
                    if tokio::fs::metadata(&path)
                        .await
                        .is_ok_and(|stat| stat.is_file() && stat.len() == track.size as u64)
                    {
                        self.record_download(&track).await?;
                        changed = true;
                    }
                }
            }
            sqlx::query("INSERT INTO cloud_downloads_scanned VALUES (?) ON CONFLICT(connection_id) DO NOTHING")
                .bind(&self.connection_id).execute(&self.db.pool).await?;
        }
        let rows: Vec<(String, String, i64)> =
            sqlx::query_as("SELECT track_id,path,size FROM cloud_downloads WHERE connection_id=?")
                .bind(&self.connection_id)
                .fetch_all(&self.db.pool)
                .await?;
        for (id, path, size) in rows {
            match tokio::fs::metadata(&path).await {
                Ok(stat) if stat.is_file() && stat.len() == size as u64 => {}
                Ok(_) => {
                    sqlx::query("DELETE FROM cloud_downloads WHERE connection_id=? AND track_id=?")
                        .bind(&self.connection_id)
                        .bind(id)
                        .execute(&self.db.pool)
                        .await?;
                    changed = true;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    sqlx::query("DELETE FROM cloud_downloads WHERE connection_id=? AND track_id=?")
                        .bind(&self.connection_id)
                        .bind(id)
                        .execute(&self.db.pool)
                        .await?;
                    changed = true;
                }
                Err(_) => {}
            }
        }
        if changed {
            self.files.publish_changed(Vec::new());
        }
        Ok(())
    }
}
