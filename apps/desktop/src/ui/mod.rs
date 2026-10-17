mod app;
pub mod components;
pub mod hooks;
pub mod pages;
pub mod routes;
mod settings;
mod shell;
pub mod theme;
mod title_bar;
mod window_manager;

use crate::services::AppServices;
use app::AudexisApp;
use freya::prelude::*;
use std::sync::Arc;
use tokio::runtime::Handle;

pub fn launch(services: Arc<AppServices>, runtime: Handle) {
    let main_window = title_bar::configure(WindowConfig::new_app(AudexisApp { services, runtime }))
        .with_title("Audexis")
        .with_size(1200., 800.)
        .with_min_size(800., 550.);

    freya::prelude::launch(
        LaunchConfig::new()
            .with_font(
                "JetBrains Mono",
                include_bytes!("../../../../shared/assets/JetBrainsMono[wght].ttf").as_slice(),
            )
            .with_default_font("JetBrains Mono")
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
