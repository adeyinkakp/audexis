use super::*;

pub struct SettingsWindow {
    pub services: Arc<AppServices>,
    pub runtime: Handle,
}

impl App for SettingsWindow {
    fn render(&self) -> impl IntoElement {
        use_provide_context({
            let services = self.services.clone();
            move || services
        });
        use_provide_context({
            let runtime = self.runtime.clone();
            move || runtime
        });
        crate::ui::theme::use_app_theme(self.services.clone());
        Settings
    }
}
