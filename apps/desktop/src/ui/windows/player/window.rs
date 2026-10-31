use super::view::PlayerView;
use crate::services::AppServices;
use freya::{
    prelude::*,
    winit::window::{WindowId, WindowLevel},
};
use std::sync::Arc;

pub(crate) fn open(
    services: Arc<AppServices>,
    runtime: tokio::runtime::Handle,
    compact: bool,
    library_window: Option<WindowId>,
) {
    let platform = Platform::get();
    if !compact {
        crate::ui::hooks::request_expanded_player();
        if let Some(library_window) = library_window {
            let close = platform.clone();
            let _ = platform.post_callback(move |current, context| {
                if let Some(window) = context.windows.get(&library_window) {
                    window.window().set_minimized(false);
                    window.window().focus_window();
                    crate::ui::windows::manager::window_manager().close_player_window(true);
                    close.close_window(current);
                }
            });
        }
        return;
    }
    let _ = platform.post_callback(move |source, context| {
        let manager = crate::ui::windows::manager::window_manager();
        if let Some(id) = manager.player_window(true) {
            if let Some(window) = context.windows.get(&id) {
                window.window().set_minimized(false);
                window.window().focus_window();
                return;
            }
        }
        let on_close = manager.clone();
        let window = WindowConfig::new_app(MiniPlayerWindow {
            services,
            runtime,
            library_window: library_window.unwrap_or(source),
        })
        .with_title("Audexis Mini Player")
        .with_size(460., 260.)
        .with_min_size(420., 250.)
        .with_window_attributes(|attributes, _| attributes.with_window_level(WindowLevel::Normal))
        .with_on_close(move |_, _| {
            on_close.close_player_window(true);
            CloseDecision::Close
        });
        let id = context.launch_window(window);
        manager.set_player_window(true, id);
    });
}

struct MiniPlayerWindow {
    services: Arc<AppServices>,
    runtime: tokio::runtime::Handle,
    library_window: WindowId,
}
impl App for MiniPlayerWindow {
    fn render(&self) -> impl IntoElement {
        let services = self.services.clone();
        let runtime = self.runtime.clone();
        use_provide_context(move || services);
        use_provide_context(move || runtime);
        crate::ui::theme::use_app_theme(self.services.clone());
        crate::ui::hooks::use_app_state(self.services.clone(), self.runtime.clone());
        PlayerView {
            compact: true,
            library_window: Some(self.library_window),
        }
    }
}
