use crate::{
    services::AppServices,
    ui::{routes::Route, theme::use_app_theme},
};
use freya::{prelude::*, router::*};
use std::sync::Arc;
use tokio::runtime::Handle;
pub struct AudexisApp {
    pub services: Arc<AppServices>,
    pub runtime: Handle,
}

impl App for AudexisApp {
    fn render(&self) -> impl IntoElement {
        let services = self.services.clone();
        let runtime = self.runtime.clone();

        use_provide_context(move || services);
        use_provide_context(move || runtime);

        use_app_theme(self.services.clone());
        super::hooks::use_app_state(self.services.clone(), self.runtime.clone());

        rect()
            .expanded()
            .theme_background()
            .theme_color()
            .maybe_child(
                (!cfg!(any(target_os = "macos", target_os = "windows")))
                    .then(ContextMenuViewer::new),
            )
            .child(Router::<Route>::new(|| {
                RouterConfig::default().with_initial_path(Route::Home)
            }))
    }
}
