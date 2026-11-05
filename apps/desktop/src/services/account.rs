use super::*;

impl AppServices {
    pub fn logout_server(&self) -> Result<Option<String>, String> {
        let mut cloud = self.cloud.lock().map_err(|_| "Connection lock poisoned")?;
        let mut previous = None;
        self.update_config(|config| previous = config.server.take())
            .map_err(|error| format!("Could not clear server settings: {error}"))?;
        *cloud = None;
        self.sync_progress.send_replace(Default::default());
        self.sync_status
            .send_replace(audexis_core::cloud_sync::CloudSyncStatus::Stopped);
        if let Ok(mut worker) = self.sync_worker.lock() {
            worker.take();
        }
        if let Some(player) = &self.player {
            if player.subscribe().borrow().id < 0
                || player
                    .subscribe()
                    .borrow()
                    .path
                    .as_deref()
                    .is_some_and(|p| p.starts_with("cloud-library://"))
            {
                let _ = player.send(audexis_audio::PlayerCmd::Stop);
            }
        }
        let _ = self.config_updates.send(());
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

    pub async fn save_server(
        self: &Arc<Self>,
        kind: ServerKind,
        url: &str,
        api_key: &str,
    ) -> Result<ServerConnection, String> {
        let url = normalize_server_url(url)?;
        let api_key = api_key.trim().to_owned();
        if api_key.is_empty() {
            return Err("Enter an API key.".into());
        }
        let identity = server_identity(kind, &url, &api_key).await?;
        let id = self
            .db
            .connection_id(&url, &identity.user_id)
            .await
            .map_err(|error| format!("Could not save account connection: {error}"))?;
        let connection = ServerConnection {
            kind,
            url,
            credential_id: Some(uuid::Uuid::new_v4().to_string()),
            connection_id: Some(id),
            server_user_id: Some(identity.user_id),
        };
        let services = self.clone();
        tokio::task::spawn_blocking(move || services.persist_server(connection, &api_key))
            .await
            .map_err(|_| "Could not save connection credentials".to_string())?
    }

    fn persist_server(
        &self,
        mut connection: ServerConnection,
        api_key: &str,
    ) -> Result<ServerConnection, String> {
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
        let _ = self.config_updates.send(());
        if let Some(previous) = previous.filter(|connection| connection.credential_id.is_some()) {
            if let Err(error) = credential_entry(&previous)
                .and_then(|entry| entry.delete_credential().map_err(|e| e.to_string()))
            {
                tracing::warn!(%error, "Could not remove previous server credential");
            }
        }
        Ok(connection)
    }
}
