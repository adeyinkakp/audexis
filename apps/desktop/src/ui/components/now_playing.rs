use super::Artwork;
use crate::ui::{
    hooks::{PlaybackPanelView, use_playback},
    theme::use_audexis_colors,
};
use audexis_audio::{PlayerCmd, RepeatMode};
use freya::{icons::lucide, prelude::*};
use std::{path::Path, sync::Arc};

#[derive(PartialEq)]
pub struct NowPlaying;
impl Component for NowPlaying {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let playback = use_playback();
        let state = playback.snapshot();
        let enabled = playback.available() && state.path.is_some();
        let services = use_consume::<Arc<crate::services::AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let router = freya::router::RouterContext::try_get();
        let busy = use_state(|| false);
        let menu_playback = playback.clone();
        let target = state
            .path
            .clone()
            .map(|path| super::media::MediaTarget::track(state.id, path));
        let song_menu = EventHandler::new(move |event: Event<PressEventData>| {
            event.stop_propagation();
            event.prevent_default();
            if let Some(target) = &target {
                crate::ui::menus::context::open(
                    &event,
                    target.clone(),
                    menu_playback.clone(),
                    services.clone(),
                    runtime.clone(),
                    router.clone(),
                    None,
                    busy,
                );
            }
        });
        let context_menu = song_menu.clone();
        let previous = playback.clone();
        let toggle = playback.clone();
        let next = playback.clone();
        let shuffle = playback.clone();
        let repeat = playback.clone();
        let lyrics_panel = playback.clone();
        let queue_panel = playback.clone();
        let panel = *playback.panel.read();
        let message = playback.error.read().clone().or(state.error.clone());
        let mut seeking = use_state(|| false);
        let repeat_label = match state.repeat_mode {
            RepeatMode::Off => "Repeat: off",
            RepeatMode::Queue => "Repeat: queue",
            RepeatMode::Track => "Repeat: track",
        };
        let icon = |name, bytes, active| {
            SvgViewer::new((name, bytes))
                .width(Size::px(15.0))
                .height(Size::px(15.0))
                .color(if active {
                    colors.primary
                } else {
                    colors.muted_foreground
                })
        };
        rect()
            .position(Position::new_absolute().bottom(16.0))
            .layer(20)
            .width(Size::fill())
            .padding((0.0, 8.0, 0.0, 8.0))
            .cross_align(Alignment::Center)
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(60.0))
                    .padding((0.0, 8.0))
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .spacing(6.0)
                    .corner_radius(12.0)
                    .background(colors.popover)
                    .border(Border::new().width(1.0).fill(colors.border))
                    .shadow(Shadow::new().y(8.0).blur(15.0).color(colors.shade))
                    .on_pointer_leave(move |_| seeking.set(false))
                    .on_secondary_down(move |event| context_menu.call(event))
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(4.0)
                            .child(control(
                                "Previous",
                                enabled,
                                icon("skip-back", lucide::skip_back(), false),
                                move |_| previous.send(PlayerCmd::Previous),
                            ))
                            .child(control(
                                if state.paused { "Resume" } else { "Pause" },
                                enabled,
                                icon(
                                    if state.paused { "play" } else { "pause" },
                                    if state.paused {
                                        lucide::play()
                                    } else {
                                        lucide::pause()
                                    },
                                    false,
                                ),
                                move |_| {
                                    toggle.send(if state.paused {
                                        PlayerCmd::Play { resuming: true }
                                    } else {
                                        PlayerCmd::Pause
                                    })
                                },
                            ))
                            .child(control(
                                "Next",
                                enabled,
                                icon("skip-forward", lucide::skip_forward(), false),
                                move |_| next.send(PlayerCmd::Next),
                            )),
                    )
                    .child(
                        rect()
                            .width(Size::flex(1.0))
                            .padding((0.0, 8.0))
                            .spacing(0.0)
                            .child(
                                rect()
                                    .width(Size::fill())
                                    .height(Size::px(40.0))
                                    .padding((2.0, 4.0))
                                    .child(TrackDetails {
                                        id: state.id,
                                        path: state.path.clone(),
                                        song_menu: song_menu.clone(),
                                    })
                                    .maybe_child((*seeking.read()).then(|| {
                                        rect()
                                            .position(Position::new_absolute().top(0.0).left(0.0))
                                            .expanded()
                                            .interactive(Interactive::No)
                                            .horizontal()
                                            .content(Content::Flex)
                                            .cross_align(Alignment::Center)
                                            .padding((0.0, 12.0))
                                            .corner_radius(8.0)
                                            .background(colors.surface_panel)
                                            .border(Border::new().width(1.0).fill(colors.highlight))
                                            .child(
                                                label()
                                                    .text(format_time(state.current_position))
                                                    .font_size(11.0),
                                            )
                                            .child(rect().width(Size::flex(1.0)))
                                            .child(
                                                label()
                                                    .text(format_time(state.duration))
                                                    .font_size(11.0),
                                            )
                                    })),
                            )
                            .child(super::compact_seek::CompactSeek {
                                position: state.current_position,
                                duration: state.duration,
                                enabled,
                                hovering: seeking,
                            }),
                    )
                    .child(
                        rect()
                            .horizontal()
                            .cross_align(Alignment::Center)
                            .spacing(4.0)
                            .child(control(
                                if state.shuffled {
                                    "Disable shuffle"
                                } else {
                                    "Enable shuffle"
                                },
                                enabled,
                                icon("shuffle", lucide::shuffle(), state.shuffled),
                                move |_| shuffle.send(PlayerCmd::ToggleShuffle),
                            ))
                            .child(control(
                                repeat_label,
                                enabled,
                                icon(
                                    repeat_label,
                                    if state.repeat_mode == RepeatMode::Track {
                                        lucide::repeat_1()
                                    } else {
                                        lucide::repeat()
                                    },
                                    state.repeat_mode != RepeatMode::Off,
                                ),
                                move |_| repeat.send(PlayerCmd::CycleRepeat),
                            ))
                            .child(control(
                                "Expand player ",
                                false,
                                icon("maximize-2", lucide::maximize_2(), false),
                                |_| {},
                            ))
                            .child(control(
                                "Lyrics",
                                true,
                                icon(
                                    "message-square-text",
                                    lucide::message_square_text(),
                                    panel == Some(PlaybackPanelView::Lyrics),
                                ),
                                move |_| lyrics_panel.toggle_panel(PlaybackPanelView::Lyrics),
                            ))
                            .child(control(
                                "Queue",
                                true,
                                icon(
                                    "list-music",
                                    lucide::list_music(),
                                    panel == Some(PlaybackPanelView::Queue),
                                ),
                                move |_| queue_panel.toggle_panel(PlaybackPanelView::Queue),
                            )),
                    ),
            )
            .maybe_child(message.map(|message| {
                label()
                    .text(message)
                    .max_lines(1)
                    .font_size(10.0)
                    .color(colors.destructive)
            }))
    }
}

fn control(
    label: &'static str,
    enabled: bool,
    icon: SvgViewer,
    action: impl FnMut(Event<PressEventData>) + 'static,
) -> impl IntoElement {
    TooltipContainer::new(Tooltip::new(label)).child(
        Button::new()
            .compact()
            .flat()
            .enabled(enabled)
            .theme_layout(ButtonLayoutThemePartial {
                width: Some(Preference::Specific(Size::px(27.0))),
                height: Some(Preference::Specific(Size::px(27.0))),
                padding: Some(Preference::Specific(Gaps::new_all(0.0))),
                ..Default::default()
            })
            .child(rect().expanded().center().a11y_alt(label).child(icon))
            .on_press(action),
    )
}
#[derive(PartialEq)]
struct TrackDetails {
    id: i64,
    path: Option<String>,
    song_menu: EventHandler<Event<PressEventData>>,
}
impl Component for TrackDetails {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let song_menu = self.song_menu.clone();
        let songs = crate::ui::hooks::use_songs(&[self.id]);
        let info = songs.data.as_ref().and_then(|songs| songs.first()).cloned();
        let fallback = self
            .path
            .as_deref()
            .and_then(|path| Path::new(path).file_stem())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Nothing is playing".into());
        let title = info
            .as_ref()
            .map(|info| info.title.clone())
            .unwrap_or(fallback);
        let subtitle = info
            .map(|info| {
                [info.artist, info.album]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · ")
            })
            .unwrap_or_else(|| {
                if self.path.is_some() {
                    "Local audio".into()
                } else {
                    String::new()
                }
            });
        rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(8.0)
            .child(Artwork {
                file_id: self.id,
                path: self.path.clone().unwrap_or_default(),
                size: 36.0,
            })
            .child(
                rect()
                    .width(Size::flex(1.0))
                    .spacing(2.0)
                    .child(
                        label()
                            .text(crate::ui::display_text(title))
                            .width(Size::fill())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(14.0)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(
                        label()
                            .text(crate::ui::display_text(subtitle))
                            .width(Size::fill())
                            .max_lines(1)
                            .text_overflow(TextOverflow::Ellipsis)
                            .font_size(12.0)
                            .color(colors.muted_foreground),
                    ),
            )
            .child(control(
                "Favorite",
                false,
                SvgViewer::new(("heart", lucide::heart()))
                    .width(Size::px(15.0))
                    .height(Size::px(15.0))
                    .color(colors.muted_foreground),
                |_| {},
            ))
            .child(control(
                "Song options",
                self.path.is_some(),
                SvgViewer::new(("song-menu", lucide::ellipsis()))
                    .width(Size::px(15.0))
                    .height(Size::px(15.0))
                    .color(colors.muted_foreground),
                move |event| song_menu.call(event),
            ))
    }
}
fn format_time(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}
