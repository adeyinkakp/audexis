use audexis_core::config::ThemePreference;
use freya::prelude::*;
use std::{io, sync::Arc};

use crate::services::AppServices;

#[derive(Clone)]
pub struct ThemeController {
    preference: State<ThemePreference>,
    services: Arc<AppServices>,
}

impl ThemeController {
    pub fn preference(&self) -> ThemePreference {
        *self.preference.read()
    }

    pub fn set(&mut self, preference: ThemePreference) -> io::Result<()> {
        self.services
            .update_config(|config| config.theme = preference)?;
        self.preference.set(preference);
        Ok(())
    }
}

fn theme_for(preference: ThemePreference) -> Theme {
    match preference {
        ThemePreference::Light => audexis_light_theme(),
        ThemePreference::Dark => audexis_dark_theme(),
        ThemePreference::System => match *Platform::get().preferred_theme.read() {
            PreferredTheme::Dark => audexis_dark_theme(),
            PreferredTheme::Light => audexis_light_theme(),
        },
    }
}

pub fn use_app_theme(services: Arc<AppServices>) {
    let preference = use_state(|| {
        services
            .user_config
            .lock()
            .expect("Settings lock poisoned")
            .theme
    });
    let updates_services = services.clone();
    use_future(move || {
        let services = updates_services.clone();
        let mut updates = services.config_updates.subscribe();
        let mut preference = preference;
        async move {
            loop {
                match updates.recv().await {
                    Ok(()) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        let value = services.user_config.lock().ok().map(|c| c.theme);
                        if let Some(value) = value {
                            preference.set_if_modified(value);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    });
    use_provide_context(move || ThemeController {
        preference,
        services,
    });
    let mut theme = use_init_theme(|| theme_for(*preference.read()));
    use_side_effect(move || theme.set(theme_for(*preference.read())));
}

pub fn use_theme_controller() -> ThemeController {
    use_consume::<ThemeController>()
}

pub const RADIUS: f32 = 8.0;
const PALETTE_KEY: &str = "audexis_palette";

mod palette;
mod widgets;
pub use palette::*;
pub use widgets::*;

pub fn use_audexis_colors() -> AudexisColors {
    let theme = use_theme();
    let current = theme.read();
    current
        .get::<AudexisColors>(PALETTE_KEY)
        .expect("Initialize theme")
        .clone()
}
