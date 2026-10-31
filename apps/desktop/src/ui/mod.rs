mod app;
pub mod components;
pub mod hooks;
mod menus;
pub mod pages;
pub mod routes;
mod settings;
mod shell;
pub mod theme;
pub mod track_info;
mod windows;

use crate::services::AppServices;
use app::AudexisApp;
use freya::prelude::*;
use std::sync::Arc;
use tokio::runtime::Handle;
use windows::{manager as window_manager, title_bar};

pub fn launch(services: Arc<AppServices>, runtime: Handle) {
    let main_window = WindowConfig::new_app(AudexisApp { services, runtime });
    let main_window = if title_bar::windows_title_bar() {
        main_window.with_decorations(false)
    } else {
        title_bar::configure(main_window)
    }
    .with_title("Audexis")
    .with_size(1200., 800.)
    .with_min_size(800., 550.);

    freya::prelude::launch(
        LaunchConfig::new()
            .with_gpu_resource_cache_limit(128 * 1024 * 1024)
            .with_window(main_window),
    );
}

pub fn launch_settings(services: Arc<AppServices>, runtime: Handle) {
    let manager = window_manager::window_manager();
    if let Some(window_id) = manager.open_settings() {
        Platform::get().focus_window(Some(window_id));
        return;
    }

    let manager_on_close = manager.clone();
    let manager_on_launch = manager.clone();
    let window = title_bar::configure(WindowConfig::new_app(settings::SettingsWindow {
        services,
        runtime,
    }))
    .with_title("Audexis Settings")
    .with_size(820., 600.)
    .with_min_size(700., 500.)
    .with_on_close(move |_, window_id| {
        manager_on_close.close_settings(window_id);
        CloseDecision::Close
    });

    let platform = Platform::get();
    let _ = platform.post_callback(move |_, context| {
        let window_id = context.launch_window(window);
        manager_on_launch.set_settings_window(window_id);
    });
}

pub(crate) fn display_text(mut text: String) -> String {
    text.retain(|character| character != '\0');
    text
}

pub(crate) fn format_time(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
