use freya::prelude::*;
use std::sync::Arc;
use tokio::runtime::Handle;

use crate::services::AppServices;
use crate::ui::{
    components::song_table::{CustomFieldsEditor, use_table_settings},
    theme::{use_audexis_colors, use_theme_controller},
};

mod appearance;
mod components;
mod library;
mod metadata;
mod unsupported;

use appearance::AppearancePanel;
use components::{Navigation, Panel, Section};
use library::LibraryPanel;
use metadata::MetadataPanel;

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

#[derive(PartialEq)]
struct Settings;

impl Component for Settings {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let section = use_state(|| Section::Appearance);
        let selected = *section.read();
        let theme = use_theme_controller();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<Handle>();
        let table_settings = use_table_settings();
        let metadata_fields = use_state(|| {
            services
                .user_config
                .lock()
                .map(|config| config.metadata_fields.clone())
                .unwrap_or_default()
        });

        rect()
            .expanded()
            .background(colors.background)
            .color(colors.foreground)
            .content(Content::Flex)
            .maybe_child(
                cfg!(target_os = "macos").then_some(crate::ui::title_bar::TitleBar {
                    title: "Audexis Settings",
                }),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.0))
                    .horizontal()
                    .child(Navigation {
                        section,
                        colors: colors.clone(),
                    })
                    .child(
                        rect()
                            .width(Size::flex(1.0))
                            .height(Size::fill())
                            .padding(32.0)
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .height(Size::fill())
                                    .overflow(Overflow::Clip)
                                    .child(Panel {
                                        section: Section::Appearance,
                                        selected,
                                        child: AppearancePanel { theme }.into_element(),
                                    })
                                    .child(Panel {
                                        section: Section::Library,
                                        selected,
                                        child: LibraryPanel {
                                            watcher: services.watcher.clone(),
                                            runtime,
                                        }
                                        .into_element(),
                                    })
                                    .child(Panel {
                                        section: Section::Metadata,
                                        selected,
                                        child: MetadataPanel {
                                            fields: metadata_fields,
                                            services: services.clone(),
                                        }
                                        .into_element(),
                                    })
                                    .child(Panel {
                                        section: Section::CustomFields,
                                        selected,
                                        child: CustomFieldsEditor {
                                            settings: table_settings,
                                        }
                                        .into_element(),
                                    })
                                    .child(Panel {
                                        section: Section::Logs,
                                        selected,
                                        child: unsupported::view(
                                            "Logs",
                                            "Backend logs are not available yet.",
                                            colors.clone(),
                                        ),
                                    }),
                            ),
                    ),
            )
    }
}
