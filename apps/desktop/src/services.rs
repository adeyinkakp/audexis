mod credentials;
pub use credentials::*;

mod account;

mod playback;

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
    pub sync_worker: Mutex<Option<audexis_core::cloud_sync::CloudWorker>>,
    pub sync_status: tokio::sync::watch::Sender<audexis_core::cloud_sync::CloudSyncStatus>,
    pub sync_progress: tokio::sync::watch::Sender<audexis_core::cloud_sync::CloudSyncProgress>,
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
    pub fn cloud_context(&self) -> Result<audexis_core::cloud_sync::CloudSyncContext, String> {
        let session = self
            .cloud
            .lock()
            .map_err(|_| "Connection lock poisoned")?
            .clone()
            .ok_or("Sign in to a server in Settings → Cloud.")?;
        Ok(audexis_core::cloud_sync::CloudSyncContext {
            db: self.db.clone(),
            connection_id: session
                .connection
                .connection_id
                .ok_or("Sign in again to verify this connection.")?,
            server_user_id: session
                .connection
                .server_user_id
                .ok_or("Sign in again to verify this account.")?,
            server_url: session.connection.url,
            api_key: session.api_key,
            cache_dir: self
                .user_config_path
                .parent()
                .ok_or("Settings directory unavailable")?
                .join("cloud-cache"),
            files: self.library_files.clone(),
        })
    }
    pub fn sync_edits(&self) {
        if let Ok(worker) = self.sync_worker.lock() {
            if let Some(worker) = worker.as_ref() {
                worker.wake_edits();
            }
        }
    }
    pub fn sync_now(&self) {
        if let Ok(worker) = self.sync_worker.lock() {
            if let Some(worker) = worker.as_ref() {
                worker.wake();
            }
        }
    }
    pub fn start_cloud_sync(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        let mut updates = self.config_updates.subscribe();
        tokio::spawn(async move {
            let mut fingerprint: Option<(String, String, bool)> = None;
            loop {
                let Some(services) = weak.upgrade() else {
                    break;
                };
                let context = services.cloud_context().ok();
                let auto = services
                    .user_config
                    .lock()
                    .map(|c| c.auto_sync)
                    .unwrap_or(false);
                let next = context
                    .as_ref()
                    .map(|c| (c.connection_id.clone(), c.api_key.clone(), auto));
                if next != fingerprint {
                    if let Ok(mut slot) = services.sync_worker.lock() {
                        slot.take();
                    }
                    if fingerprint.as_ref().map(|f| &f.0) != next.as_ref().map(|f| &f.0) {
                        if let Some(player) = &services.player {
                            if player
                                .subscribe()
                                .borrow()
                                .path
                                .as_deref()
                                .is_some_and(|p| {
                                    p.starts_with("cloud-library://")
                                        || p.starts_with("cloud-stream://")
                                })
                            {
                                let _ = player.send(audexis_audio::PlayerCmd::Stop);
                            }
                        }
                        if let Err(error) = services
                            .db
                            .set_cloud_library_connection(
                                context.as_ref().map(|c| c.connection_id.as_str()),
                            )
                            .await
                        {
                            tracing::error!(%error, "Could not switch cloud library account");
                            return;
                        }
                    }
                    if let Some(context) = &context {
                        if let Err(error) = context.refresh_library().await {
                            tracing::warn!(%error, "Could not restore cached cloud library");
                        }
                    }
                    services.library_files.publish_changed(Vec::new());
                    if let Ok(mut slot) = services.sync_worker.lock() {
                        slot.take();
                        *slot = context.map(|c| {
                            audexis_core::cloud_sync::CloudWorker::start(
                                c,
                                auto,
                                &tokio::runtime::Handle::current(),
                            )
                        });
                    }
                    fingerprint = next;
                }
                let mut status = services
                    .sync_worker
                    .lock()
                    .ok()
                    .and_then(|s| s.as_ref().map(|w| w.subscribe()));
                let mut progress = services
                    .sync_worker
                    .lock()
                    .ok()
                    .and_then(|s| s.as_ref().map(|w| w.subscribe_progress()));
                services.sync_progress.send_replace(
                    progress
                        .as_ref()
                        .map(|p| p.borrow().clone())
                        .unwrap_or_default(),
                );
                services.sync_status.send_replace(
                    status
                        .as_ref()
                        .map(|s| s.borrow().clone())
                        .unwrap_or(audexis_core::cloud_sync::CloudSyncStatus::Stopped),
                );
                let status_sender = services.sync_status.clone();
                let progress_sender = services.sync_progress.clone();
                drop(services);
                loop {
                    tokio::select! {
                        result = updates.recv() => { if matches!(result, Err(tokio::sync::broadcast::error::RecvError::Closed)) { return; } break; },
                        changed = async { if let Some(p) = progress.as_mut() {p.changed().await} else {std::future::pending().await} } => {
                            if changed.is_err() {break;}
                            if let Some(p)=progress.as_ref() {progress_sender.send_replace(p.borrow().clone());}
                        }
                        changed = async { if let Some(s) = status.as_mut() { s.changed().await } else { std::future::pending().await } } => {
                            if changed.is_err() { break; }
                            if let Some(s) = status.as_ref() { status_sender.send_replace(s.borrow().clone()); }
                        }
                    }
                }
            }
        });
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
