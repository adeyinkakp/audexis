mod library;
mod playback;

pub use library::{use_files, use_home_discovery, use_library, use_songs};
pub use playback::{Playback, PlaybackPanelView, use_playback};

pub fn use_app_state(
    services: std::sync::Arc<crate::services::AppServices>,
    runtime: tokio::runtime::Handle,
) {
    playback::provide_playback(services.clone());
    library::provide_library(services, runtime);
}

mod infinite;
pub use infinite::use_infinite_pages;
