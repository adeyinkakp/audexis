use crate::ui::{hooks::use_playback, theme::use_audexis_colors};
use audexis_audio::PlayerCmd;
use freya::prelude::*;

#[derive(PartialEq)]
pub struct CompactSeek {
    pub position: u64,
    pub duration: u64,
    pub enabled: bool,
    pub hovering: State<bool>,
}
impl Component for CompactSeek {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let playback = use_playback();
        let mut bounds = use_state(|| (0.0f64, 1.0f64));
        let mut draft = use_state(|| None::<u64>);
        let mut hovering = self.hovering;
        let a11y = use_a11y();
        let focus = use_focus(a11y);
        let duration = self.duration;
        let enabled = self.enabled && duration > 0;
        let value = (*draft.read()).unwrap_or(self.position).min(duration);
        let progress = if duration > 0 {
            value as f32 / duration as f32 * 100.0
        } else {
            0.0
        };
        let preview =
            move |x: f64| ((x / bounds.peek().1.max(1.0)).clamp(0.0, 1.0) * duration as f64) as u64;
        let release = playback.clone();
        rect()
            .width(Size::fill())
            .height(Size::px(16.0))
            .main_align(Alignment::Center)
            .a11y_id(a11y)
            .a11y_focusable(enabled)
            .a11y_role(AccessibilityRole::Slider)
            .a11y_alt(format!(
                "Seek position: {} of {} seconds",
                value / 1000,
                duration / 1000
            ))
            .on_sized(move |event: Event<SizedEventData>| {
                bounds.set((event.area.min_x() as f64, event.area.width() as f64));
            })
            .on_pointer_enter(move |_| hovering.set(true))
            .maybe(enabled, |element| {
                element
                    .on_pointer_down(move |event: Event<PointerEventData>| {
                        if !event.data().is_primary() {
                            return;
                        }
                        event.stop_propagation();
                        a11y.request_focus();
                        hovering.set(true);
                        draft.set(Some(preview(event.element_location().x)));
                    })
                    .on_global_pointer_move(move |event: Event<PointerEventData>| {
                        if draft.peek().is_some() {
                            draft.set(Some(preview(event.global_location().x - bounds.peek().0)));
                        }
                    })
                    .on_global_pointer_press(move |_: Event<PointerEventData>| {
                        if let Some(milliseconds) = draft.take() {
                            release.send(PlayerCmd::SeekMilliseconds { milliseconds });
                        }
                    })
                    .on_key_down(move |event: Event<KeyboardEventData>| {
                        let milliseconds = match event.key {
                            Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowDown) => {
                                value.saturating_sub(5000)
                            }
                            Key::Named(NamedKey::ArrowRight | NamedKey::ArrowUp) => {
                                value.saturating_add(5000).min(duration)
                            }
                            Key::Named(NamedKey::Home) => 0,
                            Key::Named(NamedKey::End) => duration,
                            _ => return,
                        };
                        event.stop_propagation();
                        hovering.set(true);
                        playback.send(PlayerCmd::SeekMilliseconds { milliseconds });
                    })
            })
            .child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(4.0))
                    .corner_radius(99.0)
                    .background(colors.control_fill)
                    .child(
                        rect()
                            .width(Size::percent(progress))
                            .height(Size::px(4.0))
                            .corner_radius(99.0)
                            .background(colors.primary),
                    ),
            )
            .maybe_child(
                (enabled
                    && (*hovering.read() || draft.read().is_some() || focus() == Focus::Keyboard))
                    .then(|| {
                        rect()
                            .position(Position::new_absolute().top(2.0))
                            .offset_x((bounds.peek().1 as f32 - 12.0).max(0.0) * progress / 100.0)
                            .width(Size::px(12.0))
                            .height(Size::px(12.0))
                            .corner_radius(99.0)
                            .background(colors.primary)
                            .border(Border::new().width(2.0).fill(colors.background))
                            .interactive(Interactive::No)
                    }),
            )
    }
}
