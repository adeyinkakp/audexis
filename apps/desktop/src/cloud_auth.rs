mod callback;
use callback::*;

use crate::services::AppServices;
use audexis_core::config::ServerKind;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub const CLOUD_API_URL: &str = match option_env!("AUDEXIS_CLOUD_API_URL") {
    Some(value) => value,
    None => "http://localhost:3000",
};
const AUTH_ORIGIN: &str = match option_env!("AUDEXIS_AUTH_ORIGIN") {
    Some(value) => value,
    None => "http://localhost:4321",
};

fn auth_url(path: &str) -> Result<reqwest::Url, String> {
    let origin =
        reqwest::Url::parse(AUTH_ORIGIN).map_err(|_| "Invalid cloud authentication URL")?;
    if origin.username() != ""
        || origin.password().is_some()
        || origin.query().is_some()
        || origin.fragment().is_some()
        || !(origin.scheme() == "https"
            || (origin.scheme() == "http"
                && matches!(origin.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))))
    {
        return Err("Cloud authentication requires HTTPS".into());
    }
    origin
        .join(path)
        .map_err(|_| "Invalid cloud authentication URL".into())
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not initialize cloud authentication".into())
}

fn random_secret() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| "Could not securely initialize sign-in")?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
}

async fn exchange_code(code: &str, verifier: &str, redirect_uri: &str) -> Result<String, String> {
    let client = http_client()?;
    let response = client
        .post(auth_url("/api/desktop/token")?)
        .json(&serde_json::json!({
            "grant_type": "authorization_code", "code": code,
            "code_verifier": verifier, "redirect_uri": redirect_uri,
        }))
        .send()
        .await
        .map_err(|_| "Could not reach Audexis. Please try signing in again.")?;
    if !response.status().is_success() {
        return Err("Sign-in expired or your subscription is inactive. Please try again.".into());
    }
    let token: TokenResponse = response
        .json()
        .await
        .map_err(|_| "Invalid sign-in response")?;
    if token.token_type != "Bearer" || !valid_token(&token.access_token) {
        return Err("Invalid sign-in credential".into());
    }
    if let Err(error) = validate_session(&token.access_token).await {
        let _ = revoke(&token.access_token).await;
        return Err(error);
    }
    Ok(token.access_token)
}

fn valid_token(token: &str) -> bool {
    token.strip_prefix("audexis_cloud_").is_some_and(|suffix| {
        suffix.len() == 48
            && suffix
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}

#[derive(serde::Deserialize)]
pub struct SessionIdentity {
    pub user_id: String,
}

#[derive(serde::Deserialize)]
struct SessionResponse {
    authenticated: bool,
    user_id: String,
}

fn session_identity(body: SessionResponse) -> Result<SessionIdentity, String> {
    if !body.authenticated
        || body.user_id.trim().is_empty()
        || body.user_id.len() > 512
        || body.user_id.chars().any(char::is_control)
    {
        return Err("The server did not return a valid account identity.".into());
    }
    Ok(SessionIdentity {
        user_id: body.user_id,
    })
}

async fn read_session(response: reqwest::Response) -> Result<SessionIdentity, String> {
    let body: SessionResponse = response.json().await.map_err(|_| {
        "The server must return authenticated and user_id in its session response.".to_string()
    })?;
    session_identity(body)
}

pub async fn custom_session(server_url: &str, key: &str) -> Result<SessionIdentity, String> {
    let url = format!("{}/api/session", server_url.trim_end_matches('/'));
    let response = http_client()?
        .get(url)
        .bearer_auth(key)
        .send()
        .await
        .map_err(|_| "Could not reach your server to verify the account.".to_string())?;
    if !response.status().is_success() {
        return Err("Could not verify this account. Your server must support GET /api/session with your API key.".into());
    }
    read_session(response).await
}

pub async fn validate_session(token: &str) -> Result<SessionIdentity, String> {
    if !valid_token(token) {
        return Err("Invalid saved cloud credential. Sign in again.".into());
    }
    let response = http_client()?
        .get(auth_url("/api/desktop/session")?)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| "Could not verify your cloud session. Check your connection.")?;
    match response.status().as_u16() {
        200 => read_session(response).await,
        403 => Err("An active Audexis Cloud subscription is required.".into()),
        401 => Err("Your cloud session has expired or was revoked. Sign in again.".into()),
        _ => Err("Could not verify your cloud session. Please try again.".into()),
    }
}

pub async fn login(cancel: tokio::sync::oneshot::Receiver<()>) -> Result<String, String> {
    wait_for_login(cancel, login_inner(), Duration::from_secs(120)).await
}

async fn wait_for_login(
    mut cancel: tokio::sync::oneshot::Receiver<()>,
    login: impl std::future::Future<Output = Result<String, String>>,
    timeout: Duration,
) -> Result<String, String> {
    tokio::select! {
        biased;
        _ = &mut cancel => Err("Sign-in cancelled.".into()),
        result = tokio::time::timeout(timeout, login) => {
            result.map_err(|_| "Login timed out. Please try again.".to_string())?
        }
    }
}

async fn login_inner() -> Result<String, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|_| "Could not start desktop sign-in callback")?;
    let host = listener
        .local_addr()
        .map_err(|_| "Could not initialize callback")?
        .to_string();
    let redirect_uri = format!("http://{host}/callback");
    let state = random_secret()?;
    let verifier = random_secret()?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut url = auth_url("/auth/desktop")?;
    url.query_pairs_mut()
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");
    open::that(url.as_str()).map_err(|_| "Could not open your browser")?;
    loop {
        let (mut stream, _) = listener
            .accept()
            .await
            .map_err(|_| "Could not accept sign-in callback")?;
        let request = tokio::time::timeout(Duration::from_secs(3), read_request(&mut stream)).await;
        let callback = request
            .ok()
            .and_then(Result::ok)
            .and_then(|request| parse_callback(&request, &host, &state).ok());
        match callback {
            None => {
                browser_response(&mut stream, "400 Bad Request", "Invalid sign-in callback").await
            }
            Some(Callback::Error(error)) => {
                let message = if error == "subscription_required" {
                    "An active Audexis Cloud subscription is required. Open your Audexis dashboard to subscribe, then sign in again."
                } else if error == "access_denied" {
                    "Sign-in cancelled"
                } else {
                    "Sign-in failed. Please try again."
                };
                browser_response(&mut stream, "403 Forbidden", message).await;
                return Err(message.into());
            }
            Some(Callback::Code(code)) => {
                let result = exchange_code(&code, &verifier, &redirect_uri).await;
                let (status, message) = if result.is_ok() {
                    ("200 OK", "Authenticated with Audexis")
                } else {
                    (
                        "403 Forbidden",
                        "Sign-in could not be completed. Return to Audexis for details.",
                    )
                };
                browser_response(&mut stream, status, message).await;
                return result;
            }
        }
    }
}

pub async fn revoke(token: &str) -> Result<(), String> {
    let response = http_client()?.post(auth_url("/api/desktop/revoke")?).bearer_auth(token)
        .send().await.map_err(|_| "Could not revoke the cloud credential. Remove this device from your Audexis dashboard when online.".to_string())?;
    if matches!(response.status().as_u16(), 204 | 401) {
        Ok(())
    } else {
        Err(
            "Cloud credential revocation failed. Remove this device from your Audexis dashboard."
                .into(),
        )
    }
}

pub async fn logout(services: Arc<AppServices>) -> Result<Option<String>, String> {
    let session = services
        .cloud
        .lock()
        .map_err(|_| "Connection lock poisoned")?
        .clone();
    let remote_warning = if let Some(session) =
        session.filter(|session| session.connection.kind == ServerKind::Cloud)
    {
        revoke(&session.api_key).await.err()
    } else {
        None
    };
    let local_warning = tokio::task::spawn_blocking(move || services.logout_server())
        .await
        .map_err(|_| "Could not finish logout")??;
    Ok(match (remote_warning, local_warning) {
        (Some(remote), Some(local)) => Some(format!("{remote} {local}")),
        (warning, None) | (None, warning) => warning,
    })
}
