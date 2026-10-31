use super::*;

pub(super) fn media_url(value: &str) -> Result<()> {
    let url = reqwest::Url::parse(value).map_err(|_| SyncError::local("Invalid media URL"))?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(SyncError::local("Invalid media URL"));
    }
    Ok(())
}
pub(super) async fn check(response: reqwest::Response) -> Result<reqwest::Response> {
    check_response(response, false).await
}
pub(super) async fn check_media(response: reqwest::Response) -> Result<reqwest::Response> {
    check_response(response, true).await
}
pub(super) async fn check_response(
    response: reqwest::Response,
    media: bool,
) -> Result<reqwest::Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let mut status = response.status().as_u16();
    let request_id = response
        .headers()
        .get("x-amz-request-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| {
            v.len() <= 128
                && v.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        })
        .map(str::to_owned);
    let mut retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut response = response;
    let mut body = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        if body.len() + chunk.len() > 8192 {
            break;
        }
        body.extend_from_slice(&chunk);
    }
    let detail = error_detail(&body);
    let text = String::from_utf8_lossy(&body);
    let code = text
        .split("<Code>")
        .nth(1)
        .and_then(|s| s.split("</Code>").next())
        .unwrap_or("");
    let cap = super::super::protocol::is_storage_cap(code, &text);
    if cap {
        status = 503;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        retry_after = retry_after.max(900.min(86400 - now % 86400 + 60));
    }
    let message = if cap {
        "Storage daily limit reached. Cloud transfers will retry after a cooldown; cached music is still available.".into()
    } else if media {
        format!(
            "Cloud media download failed ({status}){}{}",
            detail
                .map(|reason| format!(": {reason}"))
                .unwrap_or_default(),
            request_id
                .map(|id| format!("; request ID {id}"))
                .unwrap_or_default()
        )
    } else {
        match status {
            401 => "Your cloud API key was revoked. Sign in again.".into(),
            403 => "Cloud access was denied. Check your account or storage permissions.".into(),
            409 => detail.unwrap_or_else(|| {
                "This item changed on another device. Resolve the conflict in Cloud Library.".into()
            }),
            429 => detail.unwrap_or_else(|| "Server busy; waiting before retrying.".into()),
            507 => "Cloud storage is full. Free space before retrying.".into(),
            _ => match detail {
                Some(detail) => format!("Cloud request failed ({status}): {detail}"),
                None => format!("Cloud request failed ({status})"),
            },
        }
    };
    Err(SyncError {
        status: Some(status),
        message,
        retry_after,
    })
}

pub(super) fn error_detail(body: &[u8]) -> Option<String> {
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) {
        if let Some(message) = value.get("error").and_then(|v| v.as_str()) {
            return Some(
                message
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(512)
                    .collect(),
            );
        }
    }
    let text = std::str::from_utf8(body).ok()?;
    let code = text.split("<Code>").nth(1)?.split("</Code>").next()?;
    if code.is_empty()
        || code.len() > 100
        || !code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return None;
    }
    Some(format!("Object storage: {code}"))
}
