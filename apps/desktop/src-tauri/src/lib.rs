mod audio_player;
mod commands;
mod database;
mod file_watcher;
pub mod utils;
use crate::utils::errors::DatabaseError;
use audio_player::AudioPlayer;
use database::Database;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use tauri::{async_runtime, Manager};
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
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            crate::utils::errors::init_error_reporting(app.handle());
            let app_path = app.path().app_data_dir();
            if app_path.is_err() {
                panic!("No app path");
            }

            #[cfg(not(target_os = "windows"))]
            let hwnd = None;

            #[cfg(target_os = "windows")]
            let hwnd = {
                use raw_window_handle::{HasWindowHandle, RawWindowHandle};

                if let Ok(handle) = main_window.window_handle() {
                    match handle.as_raw() {
                        RawWindowHandle::Win32(win32_handle) => {
                            Some(win32_handle.hwnd.get() as *mut std::ffi::c_void)
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            };

            let config = PlatformConfig {
                dbus_name: "com.audexis",
                display_name: "My Tauri Music Player",
                hwnd,
            };

            let controls = MediaControls::new(config);

            let app_path = app_path.unwrap();
            let db_path = app_path.join("audexis_testing12.db");
            let db = async_runtime::block_on(async {
                Database::init(&db_path)
                    .await
                    .expect("Database failed to initialize")
            });
            let fw = FileWatcher::new(&app.handle());
            if fw.is_err() {
                panic!("File watcher could not b created");
            }
            let fw = fw.unwrap();
            let controls = controls.unwrap();

            app.manage(AppState {
                library_scan_lock: tauri::async_runtime::Mutex::new(()),
                db: db.clone(),
                file_watcher: Mutex::new(fw),
                audio_player: AudioPlayer::new(&app.handle(), controls, db.pool.clone())?,
                pending_worker_running: Arc::new(AtomicBool::new(false)),
                now_playing: Mutex::new(None),
            });

            if let Ok(mut watcher) = app.state::<AppState>().file_watcher.lock() {
                async_runtime::block_on(async {
                    let res: Result<(), DatabaseError> = watcher.init().await;
                    drop(res);
                });
            }
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::discovery::home_discovery::get_home_discovery,
            commands::discovery::rewind::get_rewind,
            commands::library::browse_library::browse_library,
            commands::library::favorites::get_favorite_ids,
            commands::library::favorites::set_media_loved,
            commands::library::get_files::get_media_page,
            commands::library::get_library_roots::get_library_roots,
            commands::library::get_media_files::get_media_files,
            commands::library::import_roots::import_roots,
            commands::library::rescan_library::rescan_library,
            commands::library::search_media::search_media,
            commands::library::set_library_roots::set_library_roots,
            commands::logs::get_logs,
            commands::playback::enqueue_song::enqueue_song,
            commands::playback::get_artwork::get_artwork,
            commands::playback::get_playback_modes::get_playback_modes,
            commands::playback::get_queue::get_queue,
            commands::playback::pause_playback::pause_playback,
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
