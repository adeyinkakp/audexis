use crate::services::AppServices;
use crate::ui::components::PlaylistForm;
use freya::prelude::*;
use std::sync::Arc;

pub(crate) struct NewPlaylist {
    pub(crate) services: Arc<AppServices>,
    pub(crate) runtime: tokio::runtime::Handle,
}
impl App for NewPlaylist {
    fn render(&self) -> impl IntoElement {
        let services = self.services.clone();
        let runtime = self.runtime.clone();
        use_provide_context(move || services);
        use_provide_context(move || runtime);
        crate::ui::theme::use_app_theme(self.services.clone());
        let name = use_state(String::new);
        let platform = Platform::get();
        rect()
            .expanded()
            .theme_background()
            .theme_color()
            .padding(24.)
            .spacing(16.)
            .child(label().text("New Playlist").font_size(20.))
            .child(PlaylistForm {
                name,
                on_created: EventHandler::new(move |_| {
                    let close = platform.clone();
                    platform.with_window(None, move |w| close.close_window(w.id()));
                }),
                on_cancel: None,
            })
    }
}
