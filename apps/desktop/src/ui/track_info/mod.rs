mod window;
pub use window::*;

mod operations;
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
    let window = window_config(services, runtime, tracks, tab);
    drop(Platform::get().post_callback(move |_, context| {
        context.launch_window(window);
    }));
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
        let tab_dirty = !busy
            && s.files.iter().enumerate().any(|(index, file)| {
                s.includes_file(index, active_tab)
                    && file
                        .changed_keys_matching(|key| active_tab.includes(key))
                        .next()
                        .is_some()
            });
        let validation = if busy {
            None
        } else {
            s.files
                .iter()
                .enumerate()
                .filter(|(index, _)| s.includes_file(*index, active_tab))
                .find_map(|(_, file)| {
                    metadata_editor::validate_changes_matching(file, |key| active_tab.includes(key))
                        .err()
                })
        };
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
                            .maybe_child((s.cancel.is_none() && !s.files.is_empty()).then(|| {
                                match *tab.read() {
                                    Tab::Details => tabs::Details.into_element(),
                                    Tab::Custom => tabs::Custom.into_element(),
                                    Tab::Artwork => tabs::Artwork.into_element(),
                                    Tab::Plain => tabs::Lyrics { synced: false }.into_element(),
                                    Tab::Synced => tabs::Lyrics { synced: true }.into_element(),
                                }
                            })),
                    ),
            )
            .maybe_child((!s.progress.is_empty()).then(|| {
                label()
                    .text(s.progress.clone())
                    .font_size(12.)
                    .margin((8., 24.))
            }))
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
                    .maybe_child(s.cancel.clone().map(|cancel| {
                        Button::new()
                            .enabled(!cancel.load(Ordering::Acquire))
                            .child(if cancel.load(Ordering::Acquire) {
                                "Cancelling…"
                            } else {
                                "Cancel"
                            })
                            .on_press(move |_| {
                                cancel.store(true, Ordering::Release);
                                session.write().progress =
                                    "Cancelling after the current file…".into();
                            })
                    }))
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
                                operations::save(
                                    session,
                                    save_services.clone(),
                                    runtime.clone(),
                                    active_tab,
                                );
                            }),
                    ),
            )
    }
}
