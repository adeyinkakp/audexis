mod window;
pub use window::*;

mod session;
pub use session::*;

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
                                            services.sync_edits();
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
