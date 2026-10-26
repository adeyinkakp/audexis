use super::*;

#[derive(serde::Deserialize)]
pub(super) struct Versions {
    #[serde(rename = "Version", default)]
    pub(super) versions: Vec<Version>,
    #[serde(rename = "DeleteMarker", default)]
    pub(super) markers: Vec<Version>,
}
#[derive(serde::Deserialize)]
pub(super) struct Version {
    #[serde(rename = "Key")]
    pub(super) key: String,
    #[serde(rename = "VersionId")]
    pub(super) id: String,
}

pub(super) fn validate_region(endpoint: &reqwest::Url, region: &str) -> Result<()> {
    if let Some(expected) = endpoint
        .host_str()
        .and_then(|host| host.strip_prefix("s3."))
        .and_then(|host| host.strip_suffix(".backblazeb2.com"))
    {
        if region != expected {
            return Err(Error::bad(format!(
                "B2_REGION must be {expected} to match B2_ENDPOINT (currently {region})"
            )));
        }
    }
    Ok(())
}

pub(super) fn env(name: &str) -> Result<String> {
    std::env::var(name).map_err(|_| Error::bad(format!("Missing server setting {name}")))
}
pub(super) async fn check_response(
    mut response: reqwest::Response,
    operation: &str,
) -> Result<reqwest::Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status().as_u16();
    let clock_skew = response
        .headers()
        .get("date")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| chrono::DateTime::parse_from_rfc2822(v).ok())
        .map(|time| (Utc::now().timestamp() - time.timestamp()).abs())
        .filter(|skew| *skew > 300);
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
    let mut body = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        if body.len() + chunk.len() > 8192 {
            break;
        }
        body.extend_from_slice(&chunk);
    }
    #[derive(serde::Deserialize)]
    struct Failure {
        #[serde(rename = "Code")]
        code: String,
        #[serde(rename = "Message", default)]
        message: String,
    }
    let failure = quick_xml::de::from_reader::<_, Failure>(body.as_slice()).ok();
    if failure
        .as_ref()
        .is_some_and(|e| audexis_core::cloud_sync::protocol::is_storage_cap(&e.code, &e.message))
    {
        return Err(record_cap());
    }
    let code = failure
        .map(|e| e.code)
        .filter(|code| {
            code.len() <= 100 && code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
        .unwrap_or_else(|| "UnknownStorageError".into());
    Err(Error(
        StatusCode::BAD_GATEWAY,
        format!(
            "B2 {operation} failed ({status}: {code}){}{}",
            request_id
                .map(|id| format!("; request ID {id}"))
                .unwrap_or_default(),
            clock_skew
                .map(|seconds| format!("; server clock differs from B2 by {seconds} seconds"))
                .unwrap_or_default()
        ),
    ))
}

pub(super) fn storage_error() -> Error {
    Error(
        StatusCode::BAD_GATEWAY,
        "Object storage request failed".into(),
    )
}
