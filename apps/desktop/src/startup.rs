use audexis_core::{config::UserConfig, database::Database};
use directories::ProjectDirs;
use std::{
    io,
    sync::{Arc, Mutex},
};

use crate::services::AppServices;

pub async fn initialize() -> Result<Arc<AppServices>, Box<dyn std::error::Error>> {
    let dirs = ProjectDirs::from("com", "kp", "audexis")
        .ok_or_else(|| io::Error::other("Cannot locate app directories"))?;

    let database_path = dirs.data_local_dir().join("audexis_test.db");
    let user_config_path = dirs.data_local_dir().join("user_config.json");
    let user_config = UserConfig::load_config(&user_config_path);
    let db = Database::init(&database_path).await?;

    #[cfg(not(target_os = "windows"))]
    let controls = souvlaki::MediaControls::new(souvlaki::PlatformConfig {
        display_name: "Audexis",
        dbus_name: "audexis",
        hwnd: None,
    })
    .map_err(|error| tracing::warn!(%error, "OS media controls unavailable"))
    .ok();
    #[cfg(target_os = "windows")]
    let controls = None;

    let (audio, player, audio_error) = match audexis_audio::AudioPlayer::new(
        controls,
        db.pool.clone(),
        tokio::runtime::Handle::current(),
        audexis_audio::equalizer::Settings::default(),
    ) {
        Ok(audio) => {
            let handle = audio.lock().unwrap().handle();
            (Some(audio), Some(handle), None)
        }
        Err(error) => {
            tracing::error!(%error, "Could not initialize audio");
            (None, None, Some(error.to_string()))
        }
    };

    let missing_player = player.clone();
    let library_files = audexis_core::library_files::LibraryFiles::new(
        db.pool.clone(),
        tokio::runtime::Handle::current(),
        move |path_updates, file_ids| {
            if let Some(player) = &missing_player {
                if let Err(error) = player.send(audexis_audio::PlayerCmd::ReconcileLibraryFiles {
                    path_updates,
                    file_ids,
                }) {
                    tracing::error!(%error, "Could not reconcile missing files with playback");
                }
            }
        },
    );
    audexis_core::library_files::init(&library_files);

    let (watcher, watcher_error) =
        match audexis_core::file_watcher::FileWatcher::start(db.clone(), library_files.clone())
            .await
        {
            Ok(watcher) => (Some(Arc::new(watcher)), None),
            Err(error) => {
                tracing::error!(%error, "Could not start library watcher");
                (None, Some(error.to_string()))
            }
        };
    Ok(Arc::new(AppServices {
        watcher,
        watcher_error,
        library_files,
        audio,
        player,
        audio_error,
        artwork: Arc::new(audexis_core::artwork::ArtworkCache::new(db.pool.clone())),
        db,
        user_config_path,
        config_updates: tokio::sync::broadcast::channel(16).0,
        user_config: Mutex::new(user_config),
    }))
}
