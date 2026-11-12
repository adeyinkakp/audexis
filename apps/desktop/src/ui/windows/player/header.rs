use super::open;
use crate::ui::components::player_controls::{
    ControlSize, PlaybackAction, PlaybackButton, player_button as button,
};
use crate::{
    services::AppServices,
    ui::{hooks::PlaybackPanelView, theme::use_audexis_colors},
};
use freya::{
    icons::lucide,
    prelude::*,
    winit::window::{WindowId, WindowLevel},
};
use std::sync::Arc;

#[derive(PartialEq)]
pub(super) struct PlayerHeader {
    pub compact: bool,
    pub library_window: Option<WindowId>,
    pub on_close: EventHandler<()>,
}
impl Component for PlayerHeader {
    fn render(&self) -> impl IntoElement {
        let compact = self.compact;
        let library_window = self.library_window;
        let on_close = self.on_close.clone();
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let mut pinned = use_state(|| false);
        rect()
            .width(Size::fill())
            .height(Size::px(if compact { 40. } else { 64. }))
            .padding((
                0.,
                if compact { 0. } else { 24. },
                0.,
                if !compact && cfg!(target_os = "macos") {
                    120.
                } else {
                    0.
                },
            ))
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(8.)
            .child(
                label()
                    .text("AUDEXIS")
                    .width(Size::flex(1.))
                    .max_lines(1)
                    .text_overflow(TextOverflow::Ellipsis)
                    .font_size(11.)
                    .font_weight(FontWeight::BOLD)
                    .color(colors.content_faint),
            )
            .child({
                rect()
                    .width(Size::px(76.))
                    .height(Size::px(36.))
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .padding((2., 6.))
                    .corner_radius(12.)
                    .background(colors.surface_hover)
                    .child(PlaybackButton {
                        action: PlaybackAction::Panel(PlaybackPanelView::Lyrics),
                        size: ControlSize::Mini,
                    })
                    .child(PlaybackButton {
                        action: PlaybackAction::Panel(PlaybackPanelView::Queue),
                        size: ControlSize::Mini,
                    })
            })
            .maybe_child(compact.then(|| {
                button(
                    if *pinned.read() {
                        "Unpin mini player"
                    } else {
                        "Keep on top"
                    },
                    lucide::pin(),
                    true,
                    *pinned.read(),
                    move |_| {
                        let next = !*pinned.peek();
                        pinned.set(next);
                        Platform::get().with_window(None, move |window| {
                            window.set_window_level(if next {
                                WindowLevel::AlwaysOnTop
                            } else {
                                WindowLevel::Normal
                            })
                        });
                    },
                )
            }))
            .maybe_child(compact.then(|| {
                button(
                    "Expand player in main window",
                    lucide::maximize_2(),
                    true,
                    false,
                    move |_| {
                        open(services.clone(), runtime.clone(), false, library_window);
                    },
                )
            }))
            .child(button(
                if compact {
                    "Back to library"
                } else {
                    "Collapse player (Esc)"
                },
                if compact {
                    lucide::arrow_left()
                } else {
                    lucide::chevron_down()
                },
                true,
                false,
                move |_| on_close.call(()),
            ))
    }
}
