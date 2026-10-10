mod audio_player;
mod commands;
mod database;
mod file_watcher;
pub mod utils;
use audio_player::AudioPlayer;
use database::Database;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use tauri::{async_runtime, Manager};
mod startup;
use tauri_plugin_log::{log::LevelFilter, RotationStrategy};
pub mod tag_manager;
use crate::file_watcher::FileWatcher;
use souvlaki::{MediaControls, PlatformConfig};

pub struct AppState {
    pub library_scan_lock: tauri::async_runtime::Mutex<()>,
    pub audio_player: Arc<Mutex<AudioPlayer>>,
    pub file_watcher: Mutex<FileWatcher>,
    pub db: Database,
    pub pending_worker_running: Arc<AtomicBool>,
    pub now_playing: Mutex<Option<commands::playback::shared::NowPlayingInfo>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|panic| {
        tauri_plugin_log::log::error!("panic: {panic}");
    }));
    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(commands::playback::lyrics_preview::LyricsPreviewState::default())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            crate::utils::errors::init_error_reporting(app.handle());
            crate::utils::library_files::init(app.handle());
            let error = initialize(app).err();
            if let Some(error) = &error {
                eprintln!("Startup failed: {} | {}", error.message, error.details);
                tauri_plugin_log::log::error!(
                    "Startup failed: {} | {}",
                    error.message,
                    error.details
                );
            }

            app.manage(startup::StartupStatus { error });
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            startup::get_startup_error,
            startup::relaunch_app,
            commands::discovery::home_discovery::get_home_discovery,
            commands::discovery::rewind::get_rewind,
            commands::library::browse_library::browse_library,
            commands::library::favorites::get_favorite_ids,
            commands::library::favorites::set_media_loved,
            commands::library::get_files::get_media_page,
            commands::library::get_library_roots::get_library_roots,
            commands::library::get_media_files::get_media_files,
            commands::library::import_roots::import_roots,
            commands::library::missing_files::delete_missing_file,
            commands::library::missing_files::get_missing_files,
            commands::library::missing_files::relink_missing_file,
            commands::library::rescan_library::rescan_library,
            commands::library::search_media::search_media,
            commands::library::set_library_roots::set_library_roots,
            commands::library::update_metadata::update_metadata,
            commands::library::lyrics::get_lyrics,
            commands::library::custom_fields::get_custom_fields,
            commands::library::custom_fields::get_metadata_field_catalog,
            commands::logs::get_logs,
            commands::playback::enqueue_song::enqueue_song,
            commands::playback::remove_queue_entries::remove_queue_entries,
            commands::playback::reorder_queue::reorder_queue,
            commands::playback::get_artwork::get_artwork,
            commands::playback::get_artwork::get_artwork_details,
            commands::playback::get_artwork::import_artwork,
            commands::playback::get_playback_modes::get_playback_modes,
            commands::playback::get_queue::get_queue,
            commands::playback::equalizer::get_equalizer,
            commands::playback::equalizer::set_equalizer,
            commands::playback::pause_playback::pause_playback,
            commands::playback::lyrics_preview::lyrics_preview,
            commands::playback::play::play_song,
            commands::playback::previous_song::previous_song,
            commands::playback::resume_playback::resume_playback,
            commands::playback::seek_playback::seek_playback,
            commands::playback::set_repeat_mode::set_repeat_mode,
            commands::playback::skip_song::skip_song,
            commands::playback::skip_to_index::skip_to_index,
            commands::playback::stop_playback::stop_playback,
            commands::playback::toggle_shuffle::toggle_shuffle,
            commands::playback::update_media_controls::update_media_controls,
            commands::playlists::add_playlist_track::add_playlist_track,
            commands::playlists::create_playlist::create_playlist,
            commands::playlists::delete_playlist::delete_playlist,
            commands::playlists::get_playlist::get_playlist,
            commands::playlists::get_playlists::get_playlists,
            commands::playlists::remove_playlist_track::remove_playlist_track,
            commands::playlists::rename_playlist::rename_playlist,
            commands::playlists::reorder_playlist_track::reorder_playlist_track,
            commands::tasks::get_active_tasks,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
                if let Some(state) = app.try_state::<AppState>() {
                    if let Ok(player) = state.audio_player.lock() {
                        player.finish_listening();
                    }
                }
            }
        });
}

fn initialize(app: &mut tauri::App) -> Result<(), startup::StartupError> {
    use startup::StartupError;

    let app_path = app.path().app_data_dir().map_err(|error| {
        StartupError::new(
            "Audexis could not locate its application data folder.",
            error,
        )
    })?;

    app.handle()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(LevelFilter::Warn)
                .max_file_size(2_097_152)
                .rotation_strategy(RotationStrategy::KeepSome(4))
                .build(),
        )
        .map_err(|error| {
            StartupError::new("Audexis could not initialize application logging.", error)
        })?;

    #[cfg(target_os = "windows")]
    let main_window = app.get_webview_window("main").ok_or_else(|| {
        StartupError::new(
            "Audexis could not initialize its window.",
            "Main window is unavailable",
        )
    })?;

    #[cfg(not(target_os = "windows"))]
    let hwnd = None;
    #[cfg(target_os = "windows")]
    let hwnd = main_window
        .hwnd()
        .ok()
        .map(|handle| handle.0 as *mut std::ffi::c_void);
    let controls = MediaControls::new(PlatformConfig {
        dbus_name: "com.kp.audexis",
        display_name: "Audexis",
        hwnd,
    })
    .map_err(|error| {
        StartupError::new("Audexis could not initialize system media controls.", error)
    })?;
    let db_path = app_path.join("audexis_main.db");
    let db = async_runtime::block_on(Database::init(&db_path)).map_err(|error| {
        StartupError::new("Audexis could not open its library database.", error)
    })?;
    let fw = FileWatcher::new(app.handle()).map_err(|error| {
        StartupError::new("Audexis could not create its library file watcher.", error)
    })?;
    let audio_player =
        AudioPlayer::new(app.handle(), controls, db.pool.clone()).map_err(|error| {
            StartupError::new("Audexis could not initialize audio playback.", error)
        })?;
    app.manage(AppState {
        library_scan_lock: tauri::async_runtime::Mutex::new(()),
        db,
        file_watcher: Mutex::new(fw),
        audio_player,
        pending_worker_running: Arc::new(AtomicBool::new(false)),
        now_playing: Mutex::new(None),
    });
    let state = app.state::<AppState>();
    let mut watcher = state.file_watcher.lock().map_err(|error| {
        StartupError::new(
            "Audexis could not initialize its library file watcher.",
            error,
        )
    })?;
    async_runtime::block_on(watcher.init()).map_err(|error| {
        StartupError::new(
            "Audexis could not initialize its library file watcher.",
            error,
        )
    })?;

    #[cfg(target_os = "windows")]
    main_window
        .set_decorations(false)
        .map_err(|error| StartupError::new("Audexis could not initialize its window.", error))?;
    Ok(())
}
