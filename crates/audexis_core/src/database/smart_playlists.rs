use super::Database;
use audexis_protocol::{Metadata, SmartPlaylist, Track};
use sqlx::{Row, SqliteConnection, SqlitePool};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct PlaylistSettings {
    pub name: String,
    pub smart_rules: Option<SmartPlaylist>,
    pub cloud_enabled: bool,
}
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
impl Database {
    pub async fn playlist_settings(&self, id: i64) -> Result<PlaylistSettings, String> {
        let (name, rules, cloud): (String, Option<String>, bool) =
            sqlx::query_as("SELECT name,smart_rules,cloud_enabled FROM playlists WHERE id=?")
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        Ok(PlaylistSettings {
            name,
            smart_rules: rules
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| e.to_string())?,
            cloud_enabled: cloud,
        })
    }
    pub async fn create_playlist(
        &self,
        name: &str,
        rules: Option<SmartPlaylist>,
        cloud: bool,
    ) -> Result<i64, String> {
        self.insert_playlist(name, rules, cloud, &[]).await
    }

    pub async fn create_playlist_with_tracks(
        &self,
        name: &str,
        tracks: &[i64],
        cloud: bool,
    ) -> Result<i64, String> {
        if tracks.is_empty() || tracks.len() > 10000 {
            return Err("Choose between 1 and 10,000 songs.".into());
        }
        self.insert_playlist(name, None, cloud, tracks).await
    }

    async fn insert_playlist(
        &self,
        name: &str,
        rules: Option<SmartPlaylist>,
        cloud: bool,
        tracks: &[i64],
    ) -> Result<i64, String> {
        if cloud
            && rules
                .as_ref()
                .is_some_and(SmartPlaylist::uses_listening_history)
        {
            return Err("Listening-history smart playlists use this device’s history. Save this playlist on this computer.".into());
        }
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 200 {
            return Err("Enter a playlist name between 1 and 200 characters.".into());
        }
        if let Some(rules) = &rules {
            rules.validate()?;
        }
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| e.to_string())?;
        let active: Option<String> =
            sqlx::query_scalar("SELECT connection_id FROM cloud_library_session WHERE slot=1")
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        if cloud && active.is_none() {
            return Err("Connect to a cloud account before enabling cloud sync.".into());
        }
        let id =
            sqlx::query("INSERT INTO playlists(name,smart_rules,cloud_enabled) VALUES (?,?,?)")
                .bind(name)
                .bind(
                    rules
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()
                        .map_err(|e| e.to_string())?,
                )
                .bind(cloud)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?
                .last_insert_rowid();
        if let Some(active) = active.filter(|_| cloud) {
            sqlx::query("INSERT INTO cloud_playlist_links(connection_id,playlist_id,cloud_id,snapshot,revision) VALUES (?,?,?,'',0)")
                .bind(active).bind(id).bind(uuid::Uuid::new_v4().to_string()).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
        for (order, file) in tracks.iter().enumerate() {
            let inserted = sqlx::query("INSERT INTO playlist_tracks(playlist_id,file_id,ord) SELECT ?,id,? FROM files WHERE id=? AND missing_since IS NULL")
                .bind(id).bind(order as i64).bind(file).execute(&mut *tx).await.map_err(|e| e.to_string())?;
            if inserted.rows_affected() != 1 {
                return Err("A song is no longer available. Refresh Rewind and try again.".into());
            }
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        if rules.is_some() {
            refresh_saved(&self.pool, Some(id))
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(id)
    }
    pub async fn save_playlist_settings(
        &self,
        id: i64,
        rules: Option<SmartPlaylist>,
        cloud: bool,
    ) -> Result<(), String> {
        self.save_playlist_details(id, None, rules, cloud).await
    }
    pub async fn update_playlist(
        &self,
        id: i64,
        name: &str,
        rules: Option<SmartPlaylist>,
        cloud: bool,
    ) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 200 {
            return Err("Enter a playlist name between 1 and 200 characters.".into());
        }
        self.save_playlist_details(id, Some(name), rules, cloud)
            .await
    }
    async fn save_playlist_details(
        &self,
        id: i64,
        name: Option<&str>,
        rules: Option<SmartPlaylist>,
        cloud: bool,
    ) -> Result<(), String> {
        if let Some(rules) = &rules {
            rules.validate()?;
            if cloud && rules.uses_listening_history() {
                return Err("Listening-history smart playlists use this device’s history. Save this playlist on this computer.".into());
            }
        }
        let encoded = rules
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| e.to_string())?;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| e.to_string())?;
        let active: Option<String> =
            sqlx::query_scalar("SELECT connection_id FROM cloud_library_session WHERE slot=1")
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        if cloud && active.is_none() {
            return Err("Connect to a cloud account before enabling cloud sync.".into());
        }
        if !cloud {
            let links: Vec<(String, String, i64)> = sqlx::query_as("SELECT connection_id,cloud_id,revision FROM cloud_playlist_links WHERE playlist_id=?")
                .bind(id).fetch_all(&mut *tx).await.map_err(|e| e.to_string())?;
            sqlx::query("DELETE FROM cloud_outbox WHERE json_extract(payload,'$.playlist_id')=?")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
            for (connection, remote, revision) in links {
                let operation = uuid::Uuid::new_v4().to_string();
                let job = crate::cloud_sync::store::Job::Mutate {
                    file_id: None,
                    playlist_id: None,
                    request: audexis_protocol::Mutation {
                        operation_id: operation.clone(),
                        base_revision: revision,
                        edit: audexis_protocol::Edit::DeletePlaylist {
                            playlist_id: remote.clone(),
                        },
                    },
                };
                sqlx::query("INSERT INTO cloud_outbox(operation_id,connection_id,entity_key,payload) VALUES (?,?,?,?) ON CONFLICT(connection_id,entity_key) DO NOTHING")
                    .bind(operation).bind(connection).bind(format!("playlist-delete:{remote}")).bind(serde_json::to_string(&job).map_err(|e| e.to_string())?)
                    .execute(&mut *tx).await.map_err(|e| e.to_string())?;
            }
            sqlx::query("DELETE FROM cloud_playlist_links WHERE playlist_id=?")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;
        }
        if let Some(active) = active.filter(|_| cloud) {
            let other: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cloud_playlist_links WHERE playlist_id=? AND connection_id<>?)")
                .bind(id).bind(&active).fetch_one(&mut *tx).await.map_err(|e| e.to_string())?;
            if other {
                return Err("Sign in to this playlist's account to change cloud settings.".into());
            }
            sqlx::query("INSERT INTO cloud_playlist_links(connection_id,playlist_id,cloud_id,snapshot,revision) VALUES (?,?,?,'',0) ON CONFLICT(connection_id,playlist_id) DO NOTHING")
                .bind(active).bind(id).bind(uuid::Uuid::new_v4().to_string()).execute(&mut *tx).await.map_err(|e| e.to_string())?;
        }
        let result = sqlx::query(
            "UPDATE playlists SET name=COALESCE(?,name),smart_rules=?,cloud_enabled=?,cloud_connection_id=NULL WHERE id=?",
        )
        .bind(name)
        .bind(encoded)
        .bind(cloud)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        if result.rows_affected() != 1 {
            return Err("Playlist no longer exists.".into());
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        refresh_saved(&self.pool, Some(id))
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub(crate) async fn ensure_manual(conn: &mut SqliteConnection, id: i64) -> Result<(), sqlx::Error> {
    let smart: bool =
        sqlx::query_scalar("SELECT smart_rules IS NOT NULL FROM playlists WHERE id=?")
            .bind(id)
            .fetch_one(conn)
            .await?;
    if smart {
        return Err(sqlx::Error::Protocol(
            "Smart playlists update from their rules. Edit the rules to change their songs.".into(),
        ));
    }
    Ok(())
}

pub async fn refresh(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
    refresh_saved(pool, None).await
}
async fn refresh_saved(pool: &SqlitePool, saved: Option<i64>) -> Result<bool, sqlx::Error> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM library_playlists WHERE smart_rules IS NOT NULL AND (COALESCE(json_extract(smart_rules,'$.live_updating'),1)=1 OR id=?))",
    )
    .bind(saved)
    .fetch_one(pool)
    .await?;
    if !exists {
        return Ok(false);
    }
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    let definitions: Vec<(i64, String, bool, Option<String>)> = sqlx::query_as("SELECT p.id,p.smart_rules,p.cloud_enabled,COALESCE(p.cloud_connection_id,(SELECT l.connection_id FROM cloud_playlist_links l JOIN cloud_library_session s ON s.connection_id=l.connection_id WHERE l.playlist_id=p.id LIMIT 1)) FROM library_playlists p WHERE p.smart_rules IS NOT NULL AND (COALESCE(json_extract(p.smart_rules,'$.live_updating'),1)=1 OR p.id=?)")
        .bind(saved)
        .fetch_all(&mut *tx).await?;
    if definitions.is_empty() {
        return Ok(false);
    }
    let mut changed = false;
    let mut local = Vec::new();
    if definitions.iter().any(|(_, _, cloud, _)| !cloud) {
        let rows = sqlx::query("SELECT f.id,f.file_name,COALESCE(f.format,'') AS format,f.added_at,f.duration_ms,COALESCE(m.loved,0) AS loved FROM files f LEFT JOIN media_info m ON m.file_id=f.id WHERE f.status='ok' AND f.missing_since IS NULL AND f.cloud_connection_id IS NULL")
            .fetch_all(&mut *tx).await?;
        let texts: Vec<(i64,String,String)> = sqlx::query_as("SELECT m.file_id,m.key,m.value FROM metadata_texts m JOIN files f ON f.id=m.file_id WHERE f.status='ok' AND f.missing_since IS NULL AND f.cloud_connection_id IS NULL AND m.key IN ('title','artist','albumArtist','album','genre','year') ORDER BY m.file_id,m.key,m.ord")
            .fetch_all(&mut *tx).await?;
        let mut tags = BTreeMap::<i64, audexis_protocol::Tags>::new();
        for (id, key, value) in texts {
            tags.entry(id)
                .or_default()
                .entry(key)
                .or_default()
                .push(value);
        }
        for row in rows {
            let id: i64 = row.get("id");
            local.push(Track {
                id: id.to_string(),
                revision: 0,
                sha256: String::new(),
                size: 0,
                file_name: row.get("file_name"),
                added_at: row.get("added_at"),
                format: row.get("format"),
                duration_ms: row.get("duration_ms"),
                metadata: Metadata {
                    tags: tags.remove(&id).unwrap_or_default(),
                    loved: row.get("loved"),
                },
                artwork: false,
                deleted: false,
            });
        }
    }
    let mut clouds = BTreeMap::<String, Vec<Track>>::new();
    let mut cloud_files = BTreeMap::<(String, String), i64>::new();
    for (id, encoded, cloud, connection) in definitions {
        let rules: SmartPlaylist =
            serde_json::from_str(&encoded).map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
        rules.validate().map_err(sqlx::Error::Protocol)?;
        let selected = if cloud {
            let Some(connection) = connection else {
                continue;
            };
            if !clouds.contains_key(&connection) {
                let docs: Vec<String> = sqlx::query_scalar(
                    "SELECT document FROM cloud_tracks WHERE connection_id=? ORDER BY id",
                )
                .bind(&connection)
                .fetch_all(&mut *tx)
                .await?;
                let tracks = docs
                    .iter()
                    .map(|doc| serde_json::from_str(doc))
                    .collect::<Result<Vec<Track>, _>>()
                    .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
                clouds.insert(connection.clone(), tracks);
                let linked: Vec<(String, i64)> = sqlx::query_as("SELECT l.cloud_track_id,f.id FROM files f JOIN file_cloud_links l ON l.file_id=f.id WHERE l.connection_id=? AND f.status='ok' AND f.missing_since IS NULL AND (f.cloud_connection_id IS NULL OR f.cloud_connection_id=?) ORDER BY f.cloud_connection_id IS NOT NULL,f.id")
                    .bind(&connection).bind(&connection).fetch_all(&mut *tx).await?;
                for (remote, file) in linked {
                    cloud_files
                        .entry((connection.clone(), remote))
                        .or_insert(file);
                }
            }
            let mut ids = Vec::new();
            for remote in rules.select(&clouds[&connection], now()) {
                if let Some(local) = cloud_files.get(&(connection.clone(), remote)) {
                    ids.push(*local);
                }
            }
            ids
        } else {
            let stats = if rules.uses_listening_history() {
                super::listening_stats::read(&mut tx, rules.listening_period).await?
            } else {
                BTreeMap::new()
            };
            rules
                .select_with_listening(&local, now(), |id| {
                    id.parse::<i64>()
                        .ok()
                        .and_then(|id| stats.get(&id).copied())
                        .unwrap_or_default()
                })
                .into_iter()
                .filter_map(|id| id.parse().ok())
                .collect()
        };
        let current: Vec<i64> = sqlx::query_scalar(
            "SELECT file_id FROM playlist_tracks WHERE playlist_id=? ORDER BY ord",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        if current == selected {
            continue;
        }
        changed = true;
        sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for (ord, file) in selected.iter().enumerate() {
            sqlx::query("INSERT INTO playlist_tracks(playlist_id,file_id,ord) VALUES (?,?,?)")
                .bind(id)
                .bind(file)
                .bind(ord as i64)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Ok(changed)
}
