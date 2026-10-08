use crate::tag_manager::{
    custom_fields::supported,
    tag_backend::{DefaultBackend, TagBackend},
    utils::{FrameKey, TagValue, UserTextEntry, UserUrlEntry},
};
use crate::AppState;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFields {
    supported: bool,
    supports_urls: bool,
    format: String,
    text: Vec<UserTextEntry>,
    urls: Vec<UserUrlEntry>,
    regular: std::collections::HashMap<FrameKey, Vec<String>>,
    fields: Vec<crate::tag_manager::field_catalog::FieldInfo>,
}

#[tauri::command]
pub async fn get_custom_fields(
    state: tauri::State<'_, AppState>,
    file_id: i64,
) -> Result<CustomFields, String> {
    let path = sqlx::query_scalar::<_, String>("SELECT path FROM files WHERE id = ?")
        .bind(file_id)
        .fetch_optional(&state.db.pool)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "This track no longer exists in the library".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let metadata = DefaultBackend::new()
            .read(Path::new(&path))
            .map_err(|error| super::update_metadata::error_message(&error))?;
        let regular = metadata
            .tags
            .iter()
            .filter_map(|(&key, values)| {
                if !crate::tag_manager::field_catalog::is_text(key) {
                    return None;
                }
                Some((
                    key,
                    values
                        .iter()
                        .filter_map(|value| match value {
                            TagValue::Text(text) | TagValue::Comment { text, .. } => {
                                Some(text.clone())
                            }
                            _ => None,
                        })
                        .collect(),
                ))
            })
            .collect();
        Ok(CustomFields {
            regular,
            fields: crate::tag_manager::field_catalog::catalog(Some(&metadata.tag_format)),
            supported: supported(&metadata.tag_format),
            supports_urls: crate::tag_manager::custom_fields::supports_urls(&metadata.tag_format),
            format: metadata.tag_format.to_string(),
            text: metadata
                .tags
                .get(&FrameKey::UserDefinedText)
                .into_iter()
                .flatten()
                .filter_map(|value| match value {
                    TagValue::UserText(entry) => Some(entry.clone()),
                    _ => None,
                })
                .collect(),
            urls: metadata
                .tags
                .get(&FrameKey::UserDefinedURL)
                .into_iter()
                .flatten()
                .filter_map(|value| match value {
                    TagValue::UserUrl(entry) => Some(entry.clone()),
                    _ => None,
                })
                .collect(),
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataFieldCatalog {
    fields: Vec<crate::tag_manager::field_catalog::FieldInfo>,
    custom_keys: Vec<String>,
}

#[tauri::command]
pub async fn get_metadata_field_catalog(
    state: tauri::State<'_, AppState>,
) -> Result<MetadataFieldCatalog, String> {
    let custom_keys = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT m.key FROM metadata_texts m JOIN files f ON f.id = m.file_id WHERE m.key LIKE 'custom:%' AND f.missing_since IS NULL ORDER BY m.key"
    ).fetch_all(&state.db.pool).await.map_err(|error| error.to_string())?;
    Ok(MetadataFieldCatalog {
        fields: crate::tag_manager::field_catalog::catalog(None),
        custom_keys,
    })
}
