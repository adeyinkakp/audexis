use super::super::*;
use serde_json::Value;
use std::time::Duration;

pub fn import_image(data: Vec<u8>) -> Result<TagValue, String> {
    if data.is_empty() || data.len() > 20 * 1024 * 1024 {
        return Err("Artwork must be between 1 byte and 20 MB.".into());
    }
    let format = image::guess_format(&data).map_err(|e| e.to_string())?;
    let mime = match format {
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::Png => "image/png",
        _ => return Err("Use a JPEG or PNG image.".into()),
    };
    let mut reader = image::ImageReader::new(std::io::Cursor::new(&data));
    reader.set_format(format);
    let (width, height) = reader.into_dimensions().map_err(|e| e.to_string())?;
    if u64::from(width) * u64::from(height) > 40_000_000 {
        return Err("Artwork is too large (maximum 40 megapixels).".into());
    }
    Ok(TagValue::Picture {
        mime: mime.into(),
        data,
        picture_type: Some(3),
        description: Some("Front cover".into()),
    })
}
async fn bytes(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Audexis/0.20.0")
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if data.len() + chunk.len() > 20 * 1024 * 1024 {
            return Err("Response exceeds 20 MB.".into());
        }
        data.extend_from_slice(&chunk);
    }
    Ok(data)
}
pub(super) async fn search(kind: Tab, term: String) -> Result<Vec<Value>, String> {
    let mut url = reqwest::Url::parse(if kind == Tab::Details {
        "https://itunes.apple.com/search"
    } else {
        "https://lrclib.net/api/search"
    })
    .unwrap();
    if kind == Tab::Details {
        url.query_pairs_mut()
            .append_pair("term", &term)
            .append_pair("media", "music")
            .append_pair("entity", "song")
            .append_pair("limit", "50");
    } else {
        url.query_pairs_mut().append_pair("q", &term);
    }
    let data = bytes(url.as_str()).await?;
    let value: Value = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
    let values = if kind == Tab::Details {
        value.get("results")
    } else {
        Some(&value)
    }
    .and_then(Value::as_array)
    .ok_or("Invalid search response")?;
    Ok(values
        .iter()
        .filter(|v| kind != Tab::Details || v["kind"] == "song")
        .cloned()
        .collect())
}

pub(super) async fn artwork(url: String) -> Result<TagValue, String> {
    let parsed = reqwest::Url::parse(&url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "https"
        || !parsed
            .host_str()
            .is_some_and(|host| host.ends_with(".mzstatic.com"))
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
    {
        return Err("Unsupported artwork URL".into());
    }
    import_image(bytes(&url.replace("100x100bb", "1000x1000bb")).await?)
}
