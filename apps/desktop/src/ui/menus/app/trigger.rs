use super::*;

#[derive(PartialEq)]
pub(super) struct MenuTrigger {
    pub(super) title: String,
    pub(super) enabled: bool,
    pub(super) open: State<Option<String>>,
    pub(super) hovered: State<Option<String>>,
    pub(super) edit_focus: State<Option<AccessibilityId>>,
}
impl Component for MenuTrigger {
    fn render(&self) -> impl IntoElement {
        let colors = crate::ui::theme::use_audexis_colors();
        let id = use_a11y();
        let focus = use_focus(id);
        let mut open = self.open;
        let mut hovered = self.hovered;
        let mut edit_focus = self.edit_focus;
        let enabled = self.enabled;
        let title = self.title.clone();
        let hover_title = title.clone();
        let press_title = title.clone();
        let key_title = title.clone();
        let active = open.read().as_ref() == Some(&title);
        let highlighted = active || hovered.read().as_ref() == Some(&title);
        rect()
            .height(Size::px(30.))
            .corner_radius(6.)
            .padding((0., 10.))
            .horizontal()
            .cross_align(Alignment::Center)
            .background(if highlighted {
                colors.hover
            } else {
                Color::TRANSPARENT
            })
            .border(
                Border::new()
                    .width(2.)
                    .alignment(BorderAlignment::Inner)
                    .fill(if focus() == Focus::Keyboard {
                        colors.primary
                    } else {
                        Color::TRANSPARENT
                    }),
            )
            .a11y_id(id)
            .a11y_focusable(enabled)
            .a11y_role(AccessibilityRole::MenuItem)
            .a11y_alt(title)
            .on_pointer_down(move |_| {
                if open.peek().is_none() {
                    edit_focus.set(Some(*Platform::get().focused_accessibility_id.peek()));
                }
            })
            .on_pointer_enter(move |_| {
                hovered.set(Some(hover_title.clone()));
                if enabled && open.peek().is_some() {
                    open.set(Some(hover_title.clone()));
                }
            })
            .on_pointer_leave(move |_| hovered.set(None))
            .on_key_down(move |event: Event<KeyboardEventData>| {
                if enabled
                    && event.modifiers.is_empty()
                    && matches!(
                        event.key,
                        Key::Named(NamedKey::ArrowDown | NamedKey::ArrowUp)
                    )
                {
                    event.stop_propagation();
                    event.prevent_default();
                    open.set(Some(key_title.clone()));
                }
            })
            .on_press(move |event: Event<PressEventData>| {
                event.stop_propagation();
                if enabled {
                    if open.peek().as_ref() == Some(&press_title) {
                        open.set(None);
                    } else {
                        open.set(Some(press_title.clone()));
                    }
                }
            })
            .child(
                label()
                    .text(self.title.clone())
                    .font_size(12.)
                    .color(if enabled {
                        colors.foreground
                    } else {
                        colors.muted_foreground
                    }),
            )
    }
}
