use super::*;

pub async fn load_library(
    db: Database,
    tracks: Vec<(i64, String)>,
) -> Result<Vec<EditFile>, String> {
    let mut files = Vec::new();
    for (id, path) in tracks {
        if path.starts_with("cloud-library://") {
            let mut conn = db.pool.acquire().await.map_err(|e| e.to_string())?;
            files.push(load_cloud(&mut conn, id, &path).await?);
        } else {
            files.extend(
                tokio::task::spawn_blocking(move || load(vec![(id, path)]))
                    .await
                    .map_err(|e| e.to_string())??,
            );
        }
    }
    Ok(files)
}
async fn load_cloud(
    conn: &mut sqlx::SqliteConnection,
    id: i64,
    path: &str,
) -> Result<EditFile, String> {
    use crate::tag_manager::{field_catalog, traits::Formats};
    let format: String = sqlx::query_scalar("SELECT COALESCE(format,'') FROM files WHERE id=? AND path=? AND cloud_connection_id=(SELECT connection_id FROM cloud_library_session WHERE slot=1) AND missing_since IS NULL")
        .bind(id).bind(path).fetch_optional(&mut *conn).await.map_err(|e| e.to_string())?.ok_or("This song is no longer in the active cloud library")?;
    let format = match format.as_str() {
        "FLAC" => Formats::Flac,
        "Ogg" => Formats::Ogg,
        "Itunes" => Formats::Itunes,
        "id3v2.2" => Formats::Id3v22,
        "id3v2.4" => Formats::Id3v24,
        _ => Formats::Id3v23,
    };
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key,value FROM metadata_texts WHERE file_id=? ORDER BY key,ord")
            .bind(id)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| e.to_string())?;
    let mut tags: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();
    for (key, value) in rows {
        if let Some((frame, _)) = field_catalog::FIELDS
            .iter()
            .find(|(frame, _)| frame.to_string() == key)
        {
            tags.entry(*frame).or_default().push(TagValue::Text(value));
        }
    }
    let pictures: Vec<(String, Vec<u8>, i64, String)> = sqlx::query_as(
        "SELECT mime_type,data,picture_type,description FROM metadata_pictures WHERE file_id=?",
    )
    .bind(id)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| e.to_string())?;
    for (mime, data, kind, description) in pictures {
        tags.entry(FrameKey::AttachedPicture)
            .or_default()
            .push(TagValue::Picture {
                mime,
                data,
                picture_type: u8::try_from(kind).ok(),
                description: Some(description),
            });
    }
    Ok(EditFile {
        id,
        tags: tags.clone(),
        original: MetadataFile {
            path: path.into(),
            tags,
            tag_format: format.clone(),
            tag_formats: vec![format],
            freeforms: vec![],
        },
    })
}
pub(super) async fn save_cloud(db: &Database, file: EditFile) -> Result<EditFile, String> {
    validate_changes(&file)?;
    let changes = file.changes();
    if changes.keys().any(|key| {
        matches!(
            key,
            FrameKey::AttachedPicture | FrameKey::UserDefinedText | FrameKey::UserDefinedURL
        )
    }) {
        return Err("Cloud artwork and custom fields cannot be changed in this editor yet.".into());
    }
    let path = file.original.path.display().to_string();
    let mut tx = db
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| e.to_string())?;
    let current = load_cloud(&mut tx, file.id, &path).await?;
    for key in changes.keys() {
        if current.tags.get(key) != file.original.tags.get(key) {
            return Err(format!(
                "{key} changed in the cloud. Reopen Track Info before saving."
            ));
        }
        sqlx::query("DELETE FROM metadata_texts WHERE file_id=? AND key=?")
            .bind(file.id)
            .bind(key.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
        if let Some(values) = file.tags.get(key) {
            for (ord, value) in values.iter().enumerate() {
                let text = match value {
                    TagValue::Text(text) | TagValue::Comment { text, .. } => text.clone(),
                    _ => value.to_string(),
                };
                sqlx::query("INSERT INTO metadata_texts(file_id,key,value,ord) VALUES (?,?,?,?)")
                    .bind(file.id)
                    .bind(key.to_string())
                    .bind(text)
                    .bind(ord as i64)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    use crate::cloud_sync::{
        protocol::{Edit, Mutation},
        store::{self, Job},
    };
    let (connection, track_id, revision): (String,String,i64) = sqlx::query_as("SELECT l.connection_id,l.cloud_track_id,COALESCE(s.revision,c.revision) FROM file_cloud_links l JOIN cloud_library_session active ON active.connection_id=l.connection_id JOIN cloud_tracks c ON c.connection_id=l.connection_id AND c.id=l.cloud_track_id LEFT JOIN cloud_local_state s ON s.connection_id=l.connection_id AND s.file_id=l.file_id WHERE l.file_id=?")
        .bind(file.id).fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
    let metadata = store::local_metadata(&mut tx, file.id)
        .await
        .map_err(|e| e.to_string())?;
    metadata.validate()?;
    let operation_id = uuid::Uuid::new_v4().to_string();
    let job = Job::Mutate {
        file_id: Some(file.id),
        playlist_id: None,
        request: Mutation {
            operation_id: operation_id.clone(),
            base_revision: revision,
            edit: Edit::Track { track_id, metadata },
        },
    };
    let inserted = sqlx::query("INSERT INTO cloud_outbox(operation_id,connection_id,entity_key,payload) VALUES (?,?,?,?) ON CONFLICT(connection_id,entity_key) DO NOTHING")
        .bind(operation_id).bind(connection).bind(format!("track:{}",file.id)).bind(serde_json::to_string(&job).map_err(|e|e.to_string())?).execute(&mut *tx).await.map_err(|e|e.to_string())?;
    if inserted.rows_affected() == 0 {
        return Err("An edit to this song is already syncing. Wait for it to finish or resolve its sync error before saving again.".into());
    }
    let saved = load_cloud(&mut tx, file.id, &path).await?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(saved)
}
