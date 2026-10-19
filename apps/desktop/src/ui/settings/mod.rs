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
pub(crate) use components::Section;
use components::{Navigation, Panel};

fn section_requests() -> &'static tokio::sync::watch::Sender<Section> {
    static REQUESTS: std::sync::OnceLock<tokio::sync::watch::Sender<Section>> =
        std::sync::OnceLock::new();
    REQUESTS.get_or_init(|| tokio::sync::watch::channel(Section::Appearance).0)
}
pub(crate) fn request_section(section: Section) {
    section_requests().send_replace(section);
}
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

#[derive(Clone, PartialEq)]
struct Preferences {
    theme: audexis_core::config::ThemePreference,
    density: audexis_core::config::Density,
    reduce_motion: bool,
    folders: Vec<String>,
}
#[derive(Clone)]
struct Draft {
    value: Preferences,
    saved: Preferences,
    loaded: bool,
    busy: bool,
    error: Option<String>,
}
impl Draft {
    fn dirty(&self) -> bool {
        self.value != self.saved
    }
}

#[derive(PartialEq)]
struct Settings;
impl Component for Settings {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let mut section = use_state(|| *section_requests().borrow());
        use_future(move || async move {
            let mut requests = section_requests().subscribe();
            while requests.changed().await.is_ok() {
                section.set(*requests.borrow_and_update());
            }
        });
        let selected = *section.read();
        let theme = use_theme_controller();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<Handle>();
        let mut draft = use_state(|| {
            let config = services.user_config.lock().expect("Settings lock");
            let value = Preferences {
                theme: config.theme,
                density: config.density,
                reduce_motion: config.reduce_motion,
                folders: Vec::new(),
            };
            Draft {
                saved: value.clone(),
                value,
                loaded: false,
                busy: false,
                error: None,
            }
        });
        use_provide_context(move || draft);
        let db = services.db.pool.clone();
        let rt = runtime.clone();
        use_hook(move || {
            spawn(async move {
                match rt
                    .spawn(async move { audexis_core::file_watcher::get_library_roots(&db).await })
                    .await
                {
                    Ok(Ok(folders)) => {
                        let mut d = draft.write();
                        d.value.folders = folders.clone();
                        d.saved.folders = folders;
                        d.loaded = true;
                    }
                    other => {
                        draft.write().error = Some(format!("Could not load folders: {other:?}"))
                    }
                }
            });
        });
        let table_settings = use_table_settings();
        let metadata_fields = use_state(|| {
            services
                .user_config
                .lock()
                .map(|c| c.metadata_fields.clone())
                .unwrap_or_default()
        });
        let d = draft.read().clone();
        let save_services = services.clone();
        rect()
            .expanded()
            .background(colors.background)
            .color(colors.foreground)
            .font_size(14.)
            .content(Content::Flex)
            .child({
                let heading = rect()
                    .horizontal()
                    .spacing(10.)
                    .cross_align(Alignment::Center)
                    .child(
                        SvgViewer::new(freya::icons::lucide::settings_2())
                            .width(Size::px(20.))
                            .height(Size::px(20.))
                            .stroke(colors.primary),
                    )
                    .child(label().text("Settings").font_size(18.).font_weight(FontWeight::BOLD));
                if cfg!(target_os = "macos") {
                    crate::ui::windows::title_bar::TitleBar {
                        child: heading.into_element(),
                    }
                    .into_element()
                } else {
                    rect()
                        .width(Size::fill())
                        .padding((16., 28.))
                        .child(heading)
                        .into_element()
                }
            })
            .maybe_child((!cfg!(target_os = "macos")).then(|| {
                rect()
                    .width(Size::fill())
                    .height(Size::px(1.))
                    .background(colors.border)
            }))
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::flex(1.))
                    .horizontal()
                    .content(Content::Flex)
                    .child(Navigation {
                        section,
                        colors: colors.clone(),
                    })
                    .child(
                        rect()
                            .width(Size::px(1.))
                            .height(Size::fill())
                            .background(colors.border),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.))
                            .height(Size::fill())
                            .padding(32.)
                            .overflow(Overflow::Clip)
                            .child(Panel {
                                section: Section::Appearance,
                                selected,
                                child: AppearancePanel.into_element(),
                            })
                            .child(Panel {
                                section: Section::Library,
                                selected,
                                child: LibraryPanel {
                                    watcher: services.watcher.clone(),
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
                                child: ScrollView::new()
                                    .width(Size::fill())
                                    .height(Size::fill())
                                    .child(
                                        rect()
                                            .width(Size::fill())
                                            .spacing(8.)
                                            .child(
                                                label()
                                                    .text("Saved custom fields")
                                                    .font_size(20.)
                                                    .font_weight(FontWeight::BOLD),
                                            )
                                            .child(
                                                label()
                                                    .text("Manage saved field names and column titles.")
                                                    .color(colors.muted_foreground)
                                                    .font_size(14.),
                                            )
                                            .child(CustomFieldsEditor {
                                                settings: table_settings,
                                                standalone: true,
                                            }),
                                    )
                                    .into_element(),
                            })
                            .child(Panel {
                                section: Section::Logs,
                                selected,
                                child: unsupported::view("Logs", "Backend logs are not available yet.", colors.clone()),
                            }),
                    ),
            )
            .maybe_child(d.error.clone().map(|e| {
                label()
                    .text(e)
                    .font_size(12.)
                    .color(colors.destructive)
                    .margin((8., 28.))
            }))
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(1.))
                    .background(colors.border),
            )
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .spacing(12.)
                    .padding((16., 28.))
                    .child(
                        label()
                            .text(match selected {
                                Section::Metadata => "Field selections are saved automatically.",
                                Section::CustomFields => "Saved fields apply immediately.",
                                Section::Logs => "Backend logs",
                                _ => {
                                    if d.busy {
                                        "Updating your preferences…"
                                    } else {
                                        "Changes apply when you save."
                                    }
                                }
                            })
                            .font_size(12.)
                            .color(colors.muted_foreground)
                            .width(Size::flex(1.)),
                    )
                    .maybe_child(matches!(selected, Section::Appearance | Section::Library).then(|| {
                        Button::new()
                            .flat()
                            .enabled(d.dirty() && !d.busy)
                            .child("Discard changes")
                            .on_press(move |_| {
                                let saved = draft.peek().saved.clone();
                                draft.write().value = saved;
                            })
                    }))
                    .maybe_child(matches!(selected, Section::Appearance | Section::Library).then(|| {
                        Button::new()
                            .filled()
                            .enabled(d.loaded && d.dirty() && !d.busy)
                            .child(if d.busy { "Saving…" } else { "Save changes" })
                            .on_press(move |_| {
                                let requested = draft.peek().value.clone();
                                let previous = draft.peek().saved.clone();
                                let services = save_services.clone();
                                let runtime = runtime.clone();
                                let mut theme = theme.clone();
                                draft.write().busy = true;
                                draft.write().error = None;
                                spawn(async move {
                                    let value = requested.clone();
                                    let svc = services.clone();
                                    let result = runtime
                                        .spawn(async move {
                                            if value.folders != previous.folders {
                                                let watcher =
                                                    svc.watcher.as_ref().ok_or("Library watcher is unavailable")?;
                                                for path in
                                                    value.folders.iter().filter(|p| !previous.folders.contains(p))
                                                {
                                                    watcher
                                                        .add_folder(std::path::Path::new(path))
                                                        .await
                                                        .map_err(|e| e.to_string())?;
                                                }
                                                for path in
                                                    previous.folders.iter().filter(|p| !value.folders.contains(p))
                                                {
                                                    watcher
                                                        .remove_folder(std::path::Path::new(path))
                                                        .await
                                                        .map_err(|e| e.to_string())?;
                                                }
                                            }
                                            svc.update_config(|c| {
                                                c.theme = value.theme;
                                                c.density = value.density;
                                                c.reduce_motion = value.reduce_motion;
                                            })
                                            .map_err(|e| e.to_string())
                                        })
                                        .await
                                        .map_err(|e| e.to_string())
                                        .and_then(|r| r);
                                    match result {
                                        Ok(()) => match theme.set(requested.theme) {
                                            Ok(()) => draft.write().saved = requested,
                                            Err(e) => draft.write().error = Some(e.to_string()),
                                        },
                                        Err(e) => draft.write().error = Some(e),
                                    }
                                    draft.write().busy = false;
                                });
                            })
                    })),
            )
    }
}
