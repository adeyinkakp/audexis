use base64::Engine;
use tauri::command;

use crate::AppState;

#[command]
pub async fn get_artwork(
    state: tauri::State<'_, AppState>,
    file_id: i64,
) -> Result<Option<String>, String> {
    let artwork = sqlx::query_as::<_, (Vec<u8>, String)>(
        "SELECT data, mime_type FROM metadata_pictures
         WHERE file_id = ?1 ORDER BY id LIMIT 1",
    )
    .bind(file_id)
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|error| error.to_string())?;

    Ok(artwork.map(|(data, mime)| {
        format!(
            "data:{};base64,{}",
            mime,
            base64::engine::general_purpose::STANDARD.encode(data)
        )
    }))
}
