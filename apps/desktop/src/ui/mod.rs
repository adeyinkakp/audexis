mod app;
pub mod components;
pub mod hooks;
pub mod pages;
pub mod routes;
mod shell;
pub mod theme;

use crate::services::AppServices;
use app::AudexisApp;
use freya::prelude::*;
use std::sync::Arc;
use tokio::runtime::Handle;

pub fn launch(services: Arc<AppServices>, runtime: Handle) {
    let main_window = WindowConfig::new_app(AudexisApp { services, runtime })
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
