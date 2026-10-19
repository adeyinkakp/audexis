use audexis_audio::{AudioPlayer, PlayerHandle};
use std::sync::{Arc, Mutex};

use audexis_core::{config::UserConfig, database::Database};

pub struct AppServices {
    pub audio: Option<Arc<Mutex<AudioPlayer>>>,
    pub player: Option<PlayerHandle>,
    pub audio_error: Option<String>,
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
