use audexis_audio::{AudioPlayer, PlayerHandle};
use std::sync::{Arc, Mutex};

use audexis_core::{
    config::{ServerConnection, ServerKind, UserConfig},
    database::Database,
};

#[derive(Clone)]
pub struct CloudDetails {
    pub api_key: String,
    pub connection: ServerConnection,
}
pub struct AppServices {
    pub audio: Option<Arc<Mutex<AudioPlayer>>>,
    pub player: Option<PlayerHandle>,
    pub audio_error: Option<String>,
    pub cloud: Mutex<Option<CloudDetails>>,
    pub db: Database,
    pub watcher: Option<Arc<audexis_core::file_watcher::FileWatcher>>,
    pub watcher_error: Option<String>,
    pub library_files: Arc<audexis_core::library_files::LibraryFiles>,
    pub artwork: Arc<audexis_core::artwork::ArtworkCache>,
    pub user_config_path: std::path::PathBuf,
    pub user_config: Mutex<UserConfig>,
    pub config_updates: tokio::sync::broadcast::Sender<()>,
}

impl AppServices {
    pub fn logout_server(&self) -> Result<Option<String>, String> {
        let mut cloud = self.cloud.lock().map_err(|_| "Connection lock poisoned")?;
        let mut previous = None;
        self.update_config(|config| previous = config.server.take())
            .map_err(|error| format!("Could not clear server settings: {error}"))?;
        *cloud = None;
        let Some(previous) = previous.filter(|connection| connection.credential_id.is_some())
        else {
            return Ok(None);
        };
        let cleanup =
            credential_entry(&previous).and_then(|entry| match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(error) => Err(error.to_string()),
            });
        Ok(cleanup.err().map(|error| {
            format!("Logged out, but the saved key could not be removed from keyring: {error}")
        }))
    }

    pub fn save_server(
        &self,
        kind: ServerKind,
        url: &str,
        api_key: &str,
    ) -> Result<ServerConnection, String> {
        let url = normalize_server_url(url)?;
        let api_key = api_key.trim();
        if api_key.is_empty() {
            return Err("Enter an API key.".into());
        }
        let mut connection = ServerConnection {
            kind,
            url,
            credential_id: Some(uuid::Uuid::new_v4().to_string()),
        };
        let mut cloud = self.cloud.lock().map_err(|_| "Connection lock poisoned")?;
        let stored_entry = credential_entry(&connection).and_then(|entry| {
            entry
                .set_password(api_key)
                .map_err(|error| error.to_string())?;
            Ok(entry)
        });
        let entry = match stored_entry {
            Ok(entry) => Some(entry),
            Err(error) => {
                tracing::warn!(%error, "Keyring unavailable; using a session-only server connection");
                connection.credential_id = None;
                None
            }
        };
        let mut previous = None;
        if let Err(error) = self.update_config(|config| {
            previous = config.server.replace(connection.clone());
        }) {
            if let Some(entry) = entry {
                let _ = entry.delete_credential();
            }
            return Err(format!("Could not save server settings: {error}"));
        }
        *cloud = Some(CloudDetails {
            api_key: api_key.into(),
            connection: connection.clone(),
        });
        if let Some(previous) = previous.filter(|connection| connection.credential_id.is_some()) {
            if let Err(error) = credential_entry(&previous)
                .and_then(|entry| entry.delete_credential().map_err(|e| e.to_string()))
            {
                tracing::warn!(%error, "Could not remove previous server credential");
            }
        }
        Ok(connection)
    }

    pub fn update_config(&self, edit: impl FnOnce(&mut UserConfig)) -> std::io::Result<()> {
        let mut current = self
            .user_config
            .lock()
            .map_err(|_| std::io::Error::other("Settings lock poisoned"))?;
        let mut next = current.clone();
        edit(&mut next);
        if next != *current {
            UserConfig::save_config(&self.user_config_path, &next)?;
            *current = next;
            let _ = self.config_updates.send(());
        }
        Ok(())
    }
}

fn credential_entry(connection: &ServerConnection) -> Result<keyring::Entry, String> {
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

fn normalize_server_url(value: &str) -> Result<String, String> {
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
