use super::*;

pub(super) static LIMITS: LazyLock<Mutex<HashMap<String, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
pub(super) fn epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub(super) fn limit_key(options: &StreamOptions) -> String {
    options
        .cache_dir
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| options.cache_key.clone())
}
pub(super) fn record_limit(options: &StreamOptions) {
    if let Ok(mut limits) = LIMITS.lock() {
        let now = epoch();
        limits.retain(|_, until| *until > now);
        limits.insert(
            limit_key(options),
            (now + 900).min((now / 86400 + 1) * 86400 + 60),
        );
    }
}
pub(super) fn limit_message(options: &StreamOptions) -> Option<String> {
    LIMITS
        .lock()
        .ok()?
        .get(&limit_key(options))
        .filter(|until| **until > epoch())
        .map(|_| "Storage daily limit reached. Cloud playback will retry after a cooldown.".into())
}
pub(super) fn cap_body(body: &str) -> bool {
    let body = body.to_ascii_lowercase();
    [
        "download_cap_exceeded",
        "transaction_cap_exceeded",
        "<code>cap_exceeded</code>",
        "download cap exceeded",
        "transaction cap exceeded",
        "cap exceeded",
        "cap_exceeded",
        "storage daily limit reached",
    ]
    .iter()
    .any(|value| body.contains(value))
}
