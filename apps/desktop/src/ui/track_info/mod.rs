mod lookup;
mod preview;
mod tabs;
mod text_area;
use crate::services::AppServices;
use audexis_core::{
    metadata_editor::{self, EditFile},
    tag_manager::{field_catalog, utils::*},
};
use freya::prelude::*;
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};
use tokio::runtime::Handle;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Details,
    Custom,
    Artwork,
    Plain,
    Synced,
}
impl Tab {
    fn includes(self, key: FrameKey) -> bool {
        match self {
            Self::Custom => matches!(key, FrameKey::UserDefinedText | FrameKey::UserDefinedURL),
            Self::Artwork => key == FrameKey::AttachedPicture,
            Self::Plain => key == FrameKey::UnsyncedLyrics,
            Self::Synced => key == FrameKey::SynchronizedLyrics,
            Self::Details => !matches!(
                key,
                FrameKey::UserDefinedText
                    | FrameKey::UserDefinedURL
                    | FrameKey::AttachedPicture
                    | FrameKey::UnsyncedLyrics
                    | FrameKey::SynchronizedLyrics
            ),
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Details => "Details",
            Self::Custom => "Custom Fields",
            Self::Artwork => "Artwork",
            Self::Plain => "Lyrics",
            Self::Synced => "Synchronized Lyrics",
        }
    }
}
#[derive(Clone, Default)]
pub struct Session {
    files: Vec<EditFile>,
    busy: bool,
    message: String,
    selected: usize,
    revision: u64,
}
impl Session {
    fn scoped(&self, tab: Tab) -> Vec<EditFile> {
        self.files
            .iter()
            .enumerate()
            .filter(|(index, _)| tab != Tab::Custom || *index == self.selected)
            .map(|(_, file)| {
                let mut scoped = file.clone();
                scoped.tags.retain(|key, _| tab.includes(*key));
                scoped.tags.extend(
                    file.original
                        .tags
                        .iter()
                        .filter(|(key, _)| !tab.includes(**key))
                        .map(|(key, values)| (*key, values.clone())),
                );
                scoped
            })
            .collect()
    }
    fn discard(&mut self, tab: Tab) {
        for (index, file) in self.files.iter_mut().enumerate() {
            if tab == Tab::Custom && index != self.selected {
                continue;
            }
            file.tags.retain(|key, _| !tab.includes(*key));
            file.tags.extend(
                file.original
                    .tags
                    .iter()
                    .filter(|(key, _)| tab.includes(**key))
                    .map(|(key, values)| (*key, values.clone())),
            );
        }
        self.revision += 1;
        self.message.clear();
    }
    fn accept(&mut self, mut saved: EditFile, tab: Tab) {
        if let Some(old) = self.files.iter_mut().find(|f| f.id == saved.id) {
            for key in old.changes().keys().filter(|key| !tab.includes(**key)) {
                if let Some(values) = old.tags.get(key) {
                    saved.tags.insert(*key, values.clone());
                } else {
                    saved.tags.remove(key);
                }
            }
            *old = saved;
        }
    }

    fn dirty(&self) -> bool {
        self.files.iter().any(EditFile::dirty)
    }
    fn values(&self, key: FrameKey) -> Vec<TagValue> {
        self.files
            .first()
            .and_then(|f| f.tags.get(&key))
            .cloned()
            .unwrap_or_default()
    }
    fn mixed(&self, key: FrameKey) -> bool {
        let first = self.values(key);
        self.files
            .iter()
            .any(|f| f.tags.get(&key).cloned().unwrap_or_default() != first)
    }
    fn set_values(&mut self, key: FrameKey, values: Vec<TagValue>) {
        for file in &mut self.files {
            file.tags.insert(key, values.clone());
        }
    }
    fn text(&self, key: FrameKey) -> String {
        self.values(key).first().map(text_value).unwrap_or_default()
    }
}
fn text_value(value: &TagValue) -> String {
    match value {
        TagValue::Text(s) => crate::ui::display_text(s.clone()),
        TagValue::Comment { text, .. } => crate::ui::display_text(text.clone()),
        _ => String::new(),
    }
}

pub fn open(services: Arc<AppServices>, runtime: Handle, tracks: Vec<(i64, String)>, tab: Tab) {
    let mut seen = std::collections::HashSet::new();
    let tracks: Vec<_> = tracks
        .into_iter()
        .filter(|(id, _)| seen.insert(*id))
        .collect();
    if tracks.is_empty() {
        return;
    }
    if tracks.len() > 200 {
        rfd::MessageDialog::new()
            .set_title("Track Info")
            .set_description("Select at most 200 tracks to edit at once.")
            .show();
        return;
    }
    let window = window_config(services, runtime, tracks, tab);
    let _ = Platform::get().post_callback(move |_, context| {
        context.launch_window(window);
    });
}
pub fn window_config(
    services: Arc<AppServices>,
    runtime: Handle,
    tracks: Vec<(i64, String)>,
    tab: Tab,
) -> WindowConfig {
    let guard = Arc::new(AtomicU8::new(0));
    let close_guard = guard.clone();
    crate::ui::windows::title_bar::configure(WindowConfig::new_app(EditorApp {
        services,
        runtime,
        tracks,
        tab,
        guard,
    }))
    .with_title("Track Info")
    .with_size(1000., 800.)
    .with_min_size(800., 600.)
    .with_on_close(move |_, _| match close_guard.load(Ordering::Acquire) {
        2 => CloseDecision::KeepOpen,
        1 => {
            let discard = rfd::MessageDialog::new()
                .set_title("Unsaved metadata")
                .set_description("Discard your unsaved changes and close Track Info?")
                .set_buttons(rfd::MessageButtons::YesNo)
                .show();
            if discard == rfd::MessageDialogResult::Yes {
                CloseDecision::Close
            } else {
                CloseDecision::KeepOpen
            }
        }
        _ => CloseDecision::Close,
    })
}

struct EditorApp {
    services: Arc<AppServices>,
    runtime: Handle,
    tracks: Vec<(i64, String)>,
    tab: Tab,
    guard: Arc<AtomicU8>,
}
impl App for EditorApp {
    fn render(&self) -> impl IntoElement {
        use_provide_context({
            let services = self.services.clone();
            move || services
        });
        use_provide_context({
            let runtime = self.runtime.clone();
            move || runtime
        });
        super::theme::use_app_theme(self.services.clone());
        let mut session = use_state(|| Session {
            busy: true,
            ..Session::default()
        });
        use_hook({
            let tracks = self.tracks.clone();
            let runtime = self.runtime.clone();
            move || {
                spawn(async move {
                    let result = runtime
                        .spawn_blocking(move || metadata_editor::load(tracks))
                        .await;
                    let mut s = session.write();
                    s.busy = false;
                    match result {
                        Ok(Ok(files)) => s.files = files,
                        other => s.message = format!("Could not read metadata: {other:?}"),
                    }
                });
            }
        });
        use_provide_context(move || session);
        let current = session.read();
        self.guard.store(
            if current.busy {
                2
            } else if current.dirty() {
                1
            } else {
                0
            },
            Ordering::Release,
        );
        drop(current);
        Editor {
            initial_tab: self.tab,
        }
    }
}
#[derive(PartialEq)]
struct Editor {
    initial_tab: Tab,
}
impl Component for Editor {
    fn render(&self) -> impl IntoElement {
        let mut session = use_consume::<State<Session>>();
        let mut tab = use_state(|| self.initial_tab);
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<Handle>();
        let colors = super::theme::use_audexis_colors();
        let s = session.read().clone();
        let busy = s.busy;
        let active_tab = *tab.read();
        let scoped = s.scoped(active_tab);
        let tab_dirty = scoped.iter().any(EditFile::dirty);
        let validation = scoped
            .iter()
            .find_map(|file| metadata_editor::validate_changes(file).err());
        let save_services = services.clone();
        rect()
            .expanded()
            .content(Content::Flex)
            .background(colors.background)
            .color(colors.foreground)
            .font_size(14.)
            .maybe_child(cfg!(target_os = "macos").then_some(
                crate::ui::windows::title_bar::TitleBar {
                    child: label().text("Track Info").into_element(),
                },
            ))
            .child(
                rect()
                    .horizontal()
                    .padding((12., 24.))
                    .spacing(8.)
                    .children(
                        [
                            Tab::Details,
                            Tab::Custom,
                            Tab::Artwork,
                            Tab::Plain,
                            Tab::Synced,
                        ]
                        .into_iter()
                        .map(move |item| {
                            Button::new()
                                .flat()
                                .theme_colors(ButtonColorsThemePartial {
                                    background: Some(
                                        (if active_tab == item {
                                            colors.primary_washed
                                        } else {
                                            colors.background
                                        })
                                        .into(),
                                    ),
                                    color: Some(
                                        (if active_tab == item {
                                            colors.primary
                                        } else {
                                            colors.muted_foreground
                                        })
                                        .into(),
                                    ),
                                    ..Default::default()
                                })
                                .child(item.title())
                                .on_press(move |_| tab.set(item))
                                .into_element()
                        }),
                    ),
            )
            .child(
                ScrollView::new()
                    .height(Size::flex(1.))
                    .width(Size::fill())
                    .child(
                        rect()
                            .width(Size::fill())
                            .padding(24.)
                            .spacing(12.)
                            .maybe_child(s.files.is_empty().then(|| {
                                label().text(if busy {
                                    "Loading metadata…"
                                } else {
                                    "No metadata loaded."
                                })
                            }))
                            .maybe_child((!s.files.is_empty()).then(|| match *tab.read() {
                                Tab::Details => tabs::Details.into_element(),
                                Tab::Custom => tabs::Custom.into_element(),
                                Tab::Artwork => tabs::Artwork.into_element(),
                                Tab::Plain => tabs::Lyrics { synced: false }.into_element(),
                                Tab::Synced => tabs::Lyrics { synced: true }.into_element(),
                            })),
                    ),
            )
            .maybe_child((!s.message.is_empty()).then(|| {
                label()
                    .text(s.message.clone())
                    .font_size(12.)
                    .margin((8., 24.))
            }))
            .maybe_child(
                validation
                    .clone()
                    .map(|message| label().text(message).font_size(12.).margin(12.)),
            )
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .padding((16., 24.))
                    .background(colors.surface_subtle)
                    .spacing(8.)
                    .maybe_child(matches!(active_tab, Tab::Plain | Tab::Synced).then(|| {
                        Button::new()
                            .flat()
                            .enabled(!busy)
                            .child("Clear")
                            .on_press(move |_| {
                                session.write().set_values(
                                    if active_tab == Tab::Plain {
                                        FrameKey::UnsyncedLyrics
                                    } else {
                                        FrameKey::SynchronizedLyrics
                                    },
                                    vec![],
                                );
                            })
                    }))
                    .child(rect().width(Size::flex(1.)))
                    .child(
                        Button::new()
                            .enabled(tab_dirty && !busy)
                            .child("Discard changes")
                            .on_press(move |_| {
                                let mut s = session.write();
                                s.discard(active_tab);
                            }),
                    )
                    .child(
                        Button::new()
                            .filled()
                            .enabled(tab_dirty && !busy && validation.is_none())
                            .child(if busy {
                                "Saving…".to_string()
                            } else {
                                format!("Save {}", active_tab.title())
                            })
                            .on_press(move |_| {
                                let files = session.peek().scoped(active_tab);
                                let services = save_services.clone();
                                let runtime = runtime.clone();
                                session.write().busy = true;
                                spawn(async move {
                                    let db = services.db.clone();
                                    let result = runtime
                                        .spawn(
                                            async move { metadata_editor::save(db, files).await },
                                        )
                                        .await;
                                    match result {
                                        Ok(result) => {
                                            let ids = result.saved.iter().map(|f| f.id).collect();
                                            services.library_files.publish_changed(ids);
                                            let cache = services.artwork.clone();
                                            let _ = runtime
                                                .spawn(async move {
                                                    cache.invalidate().await;
                                                })
                                                .await;
                                            let mut s = session.write();
                                            let count = result.saved.len();
                                            for file in result.saved {
                                                s.accept(file, active_tab);
                                            }
                                            s.message = if result.failures.is_empty() {
                                                format!("Saved {count} file(s).")
                                            } else {
                                                format!(
                                                    "Saved {count}; {} failed. {}",
                                                    result.failures.len(),
                                                    result.failures.join("\n")
                                                )
                                            };
                                            s.revision += 1;
                                        }
                                        Err(e) => session.write().message = e.to_string(),
                                    }
                                    session.write().busy = false;
                                });
                            }),
                    ),
            )
    }
}
