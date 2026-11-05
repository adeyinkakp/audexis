use crate::{
    AppState,
    error::{Error, Result},
};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Account {
    pub user_id: String,
    pub quota: i64,
    pub local: bool,
}
#[derive(Deserialize)]
struct Identity {
    authenticated: bool,
    user_id: String,
}
#[derive(Default)]
pub struct Limits {
    entries: Mutex<HashMap<String, (Instant, u32)>>,
}
impl Limits {
    pub fn check(&self, key: String, limit: u32) -> Result<()> {
        let mut entries = self.entries.lock().unwrap();
        let now = Instant::now();
        if entries.len() > 10000 {
            entries.retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(60))
        }
        if entries.len() > 20000 && !entries.contains_key(&key) {
            return Err(Error(
                StatusCode::TOO_MANY_REQUESTS,
                "Server is busy; retry in 60 seconds".into(),
            ));
        }
        let entry = entries.entry(key).or_insert((now, 0));
        if now.duration_since(entry.0) >= Duration::from_secs(60) {
            *entry = (now, 0)
        }
        entry.1 = entry.1.saturating_add(1);
        if entry.1 > limit {
            return Err(Error(
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded; retry in 60 seconds".into(),
            ));
        }
        Ok(())
    }
}
pub async fn identify(state: &AppState, headers: &HeaderMap) -> Result<Account> {
    let token = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|s| !s.is_empty() && s.len() < 1024)
        .ok_or_else(|| {
            Error(
                StatusCode::UNAUTHORIZED,
                "A valid API key is required".into(),
            )
        })?;
    let hash = format!("{:x}", Sha256::digest(token.as_bytes()));
    state.limits.check(format!("auth:{hash}"), 360)?;
    let local: Option<(String, i64, bool)> = sqlx::query_as("SELECT u.user_id,a.quota_bytes,u.disabled FROM self_hosted_users u JOIN cloud_accounts a ON a.user_id=u.user_id WHERE u.api_key_sha256=$1")
        .bind(&hash).fetch_optional(&state.db).await?;
    if let Some((user_id, quota, disabled)) = local {
        if disabled {
            return Err(Error(
                StatusCode::FORBIDDEN,
                "This server account is disabled".into(),
            ));
        }
        return Ok(Account {
            user_id,
            quota,
            local: true,
        });
    }
    if token.starts_with("audexis_sh_") {
        return Err(Error(
            StatusCode::UNAUTHORIZED,
            "Invalid or revoked API key".into(),
        ));
    }
    let Some(origin) = &state.auth_origin else {
        return Err(Error(StatusCode::UNAUTHORIZED, "Invalid API key".into()));
    };
    let response = state
        .client
        .get(format!("{origin}/api/desktop/session"))
        .bearer_auth(token)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|_| {
            Error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Account verification is temporarily unavailable".into(),
            )
        })?;
    match response.status().as_u16() {
        401 => {
            return Err(Error(
                StatusCode::UNAUTHORIZED,
                "Invalid or revoked API key".into(),
            ));
        }
        403 => {
            return Err(Error(
                StatusCode::FORBIDDEN,
                "An active subscription is required".into(),
            ));
        }
        200 => {}
        _ => {
            return Err(Error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Account verification is temporarily unavailable".into(),
            ));
        }
    }
    let identity: Identity = response.json().await.map_err(|_| {
        Error(
            StatusCode::BAD_GATEWAY,
            "Invalid account verification response".into(),
        )
    })?;
    if !identity.authenticated || identity.user_id.trim().is_empty() || identity.user_id.len() > 512
    {
        return Err(Error(
            StatusCode::UNAUTHORIZED,
            "Invalid account identity".into(),
        ));
    }
    Ok(Account {
        user_id: identity.user_id,
        quota: state.default_quota,
        local: false,
    })
}
pub async fn middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    let peer = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|p| p.0.ip());
    let peer = if peer.is_some_and(|ip| state.trusted_proxies.contains(&ip)) {
        let forwarded = request
            .headers()
            .get_all("x-forwarded-for")
            .iter()
            .map(|value| {
                value
                    .to_str()
                    .map_err(|_| Error::bad("Invalid forwarded client address"))
            })
            .collect::<Result<Vec<_>>>()?
            .join(",");
        let addresses = forwarded
            .split(',')
            .map(str::trim)
            .map(str::parse::<std::net::IpAddr>)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| Error::bad("A trusted proxy must supply X-Forwarded-For"))?;
        if addresses.is_empty() || addresses.len() > 32 {
            return Err(Error::bad("Invalid forwarded client address"));
        }
        addresses
            .iter()
            .rev()
            .find(|ip| !state.trusted_proxies.contains(ip))
            .or_else(|| addresses.first())
            .unwrap()
            .to_string()
    } else {
        peer.map(|ip| ip.to_string())
            .unwrap_or_else(|| "in-process".into())
    };
    state.limits.check(format!("peer:{peer}"), 1800)?;
    state.limits.check("auth-global".into(), 10000)?;
    let account = identify(&state, request.headers()).await?;
    let upload = request.uri().path().ends_with("/uploads");
    state.limits.check(
        format!(
            "user:{}:{}",
            account.user_id,
            if upload { "uploads" } else { "api" }
        ),
        if upload { 120 } else { 300 },
    )?;
    if !account.local {
        let saved: Option<i64> =
            sqlx::query_scalar("SELECT quota_bytes FROM cloud_accounts WHERE user_id=$1")
                .bind(&account.user_id)
                .fetch_optional(&state.db)
                .await?;
        if saved != Some(account.quota) {
            sqlx::query("INSERT INTO cloud_accounts(user_id,quota_bytes) VALUES ($1,$2) ON CONFLICT(user_id) DO UPDATE SET quota_bytes=excluded.quota_bytes WHERE cloud_accounts.quota_bytes IS DISTINCT FROM excluded.quota_bytes").bind(&account.user_id).bind(account.quota).execute(&state.db).await?;
        }
    }
    request.extensions_mut().insert(account);
    Ok(next.run(request).await)
}
