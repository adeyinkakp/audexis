use super::*;

pub(super) fn credential_entry(connection: &ServerConnection) -> Result<keyring::Entry, String> {
    let id = connection
        .credential_id
        .as_deref()
        .ok_or("This connection is session-only.")?;
    keyring::Entry::new("com.kp.audexis", id).map_err(|e| format!("Could not access keyring: {e}"))
}

pub fn restore_server(config: &UserConfig) -> Result<Option<CloudDetails>, String> {
    config
        .server
        .as_ref()
        .filter(|connection| connection.credential_id.is_some())
        .map(|connection| {
            let api_key = credential_entry(connection)?
                .get_password()
                .map_err(|e| format!("Could not read server API key from keyring: {e}"))?;
            Ok(CloudDetails {
                api_key,
                connection: connection.clone(),
            })
        })
        .transpose()
}

pub(crate) fn normalize_server_url(value: &str) -> Result<String, String> {
    let mut url = reqwest::Url::parse(value.trim()).map_err(|_| "Enter a valid server URL.")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "Use an HTTP or HTTPS server URL without credentials, a query, or a fragment.".into(),
        );
    }
    let path = url.path().trim_end_matches('/').to_owned();
    url.set_path(&path);
    Ok(url.to_string().trim_end_matches('/').to_owned())
}

pub(super) async fn server_identity(
    kind: ServerKind,
    url: &str,
    key: &str,
) -> Result<crate::cloud_auth::SessionIdentity, String> {
    match kind {
        ServerKind::Cloud => crate::cloud_auth::validate_session(key).await,
        ServerKind::Custom => crate::cloud_auth::custom_session(url, key).await,
    }
}

pub async fn resolve_restored_connection(
    db: &Database,
    session: &mut CloudDetails,
) -> Result<(), String> {
    let url = normalize_server_url(&session.connection.url)?;
    if let (Some(_), Some(user)) = (
        &session.connection.connection_id,
        &session.connection.server_user_id,
    ) {
        let client = audexis_core::cloud_sync::CloudClient::new(&url, &session.api_key)
            .map_err(|e| e.to_string())?;
        match client.verify(user).await {
            Ok(()) => {}
            Err(e) if matches!(e.status, Some(401 | 403)) => return Err(e.to_string()),
            Err(_) => {
                tracing::info!("Server unavailable; keeping verified account for offline browsing")
            }
        }
        session.connection.connection_id = Some(
            db.connection_id(&url, user)
                .await
                .map_err(|e| e.to_string())?,
        );
        session.connection.url = url;
        return Ok(());
    }
    let identity = server_identity(session.connection.kind, &url, &session.api_key).await?;
    if session
        .connection
        .server_user_id
        .as_deref()
        .is_some_and(|saved| saved != identity.user_id)
    {
        return Err(
            "The saved credential belongs to a different account. Please sign in again.".into(),
        );
    }
    let id = db
        .connection_id(&url, &identity.user_id)
        .await
        .map_err(|error| format!("Could not restore account connection: {error}"))?;
    session.connection.url = url;
    session.connection.connection_id = Some(id);
    session.connection.server_user_id = Some(identity.user_id);
    Ok(())
}
