mod details;
use details::*;

use super::{
    Artwork,
    player_controls::{
        ControlSize, PlaybackAction, PlaybackButton, PlaybackTimes, player_button_sized,
    },
};
use crate::ui::{
    hooks::{PlaybackPanelView, use_playback},
    theme::use_audexis_colors,
};
use freya::{icons::lucide, prelude::*};
use std::sync::Arc;

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
            .filter(|_| state.id > 0)
            .map(|path| super::media::MediaTarget::track(state.id, path));
        let fullscreen_services = services.clone();
        let fullscreen_runtime = runtime.clone();
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
        let mini_services = fullscreen_services.clone();
        let mini_runtime = fullscreen_runtime.clone();
        let message = playback.error.read().clone().or(state.error.clone());
        let mut seeking = use_state(|| false);
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
                            .spacing(4.)
                            .child(PlaybackButton {
                                action: PlaybackAction::Previous,
                                size: ControlSize::Bar,
                            })
                            .child(PlaybackButton {
                                action: PlaybackAction::Toggle,
                                size: ControlSize::Bar,
                            })
                            .child(PlaybackButton {
                                action: PlaybackAction::Next,
                                size: ControlSize::Bar,
                            }),
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
                                            .child(PlaybackTimes {
                                                position: state.current_position,
                                                duration: state.duration,
                                                muted: false,
                                            })
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
                            .spacing(4.)
                            .child(PlaybackButton {
                                action: PlaybackAction::Shuffle,
                                size: ControlSize::Bar,
                            })
                            .child(PlaybackButton {
                                action: PlaybackAction::Repeat,
                                size: ControlSize::Bar,
                            })
                            .child(player_button_sized(
                                "Fullscreen player",
                                lucide::maximize_2(),
                                true,
                                false,
                                ControlSize::Bar,
                                move |_| {
                                    crate::ui::windows::player::open(
                                        fullscreen_services.clone(),
                                        fullscreen_runtime.clone(),
                                        false,
                                        None,
                                    )
                                },
                            ))
                            .child(player_button_sized(
                                "Mini player",
                                lucide::picture_in_picture_2(),
                                true,
                                false,
                                ControlSize::Bar,
                                move |_| {
                                    crate::ui::windows::player::open(
                                        mini_services.clone(),
                                        mini_runtime.clone(),
                                        true,
                                        None,
                                    )
                                },
                            ))
                            .child(PlaybackButton {
                                action: PlaybackAction::Panel(PlaybackPanelView::Lyrics),
                                size: ControlSize::Bar,
                            })
                            .child(PlaybackButton {
                                action: PlaybackAction::Panel(PlaybackPanelView::Queue),
                                size: ControlSize::Bar,
                            }),
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
