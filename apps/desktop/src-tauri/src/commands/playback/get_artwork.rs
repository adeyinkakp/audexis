use base64::Engine;
use serde::Serialize;
use tauri::command;

use crate::AppState;

#[derive(Serialize)]
pub struct ArtworkInfo {
    id: i64,
    file_id: i64,
    data_url: String,
    data_base64: String,
    mime_type: String,
    picture_type: i64,
    description: String,
}

#[derive(Serialize)]
pub struct ImportedArtwork {
    mime: String,
    data_base64: String,
    picture_type: u8,
    description: String,
}

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

#[command]
pub async fn get_artwork_details(
    state: tauri::State<'_, AppState>,
    file_ids: Vec<i64>,
) -> Result<Vec<ArtworkInfo>, String> {
    let mut artwork = Vec::new();

    for file_id in file_ids {
        let rows = sqlx::query_as::<_, (i64, Vec<u8>, String, i64, String)>(
            "SELECT id, data, mime_type, picture_type, description
             FROM metadata_pictures
             WHERE file_id = ?1
             ORDER BY id",
        )
        .bind(file_id)
        .fetch_all(&state.db.pool)
        .await
        .map_err(|error| error.to_string())?;

        artwork.extend(
            rows.into_iter()
                .map(|(id, data, mime_type, picture_type, description)| {
                    let data_base64 = base64::engine::general_purpose::STANDARD.encode(data);
                    ArtworkInfo {
                        id,
                        file_id,
                        data_url: format!("data:{};base64,{}", mime_type, data_base64),
                        data_base64,
                        mime_type,
                        picture_type,
                        description,
                    }
                }),
        );
    }

    Ok(artwork)
}

#[command]
pub fn import_artwork() -> Result<Option<ImportedArtwork>, String> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Select artwork")
        .add_filter("Images", &["jpg", "jpeg", "png", "gif", "webp", "bmp"])
        .pick_file()
    else {
        return Ok(None);
    };

    let mime = match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        _ => return Err("Unsupported artwork format".into()),
    };
    let data = std::fs::read(&path).map_err(|error| error.to_string())?;
    if data.len() > 25 * 1024 * 1024 {
        return Err("Artwork must be smaller than 25 MB".into());
    }

    Ok(Some(ImportedArtwork {
        mime: mime.into(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(data),
        picture_type: 3,
        description: String::new(),
    }))
}
