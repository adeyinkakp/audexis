use super::*;

#[derive(PartialEq)]
pub(crate) struct KeyboardMenu {
    pub child: Element,
}
impl Component for KeyboardMenu {
    fn render(&self) -> impl IntoElement {
        let MenuGroup { group_id } = use_consume::<MenuGroup>();
        let previous = use_hook(|| *Platform::get().focused_accessibility_id.peek());
        use_side_effect(move || group_id.request_focus());
        use_future(move || async move {
            let mut ticker = RenderingTicker::get();
            ticker.tick().await;
            if group_id.is_focused() {
                Platform::get().send(UserEvent::FocusAccessibilityNode(
                    AccessibilityFocusStrategy::Forward(AccessibilityFocusMovement::InsideGroup),
                ));
            }
        });
        use_drop(move || {
            let role = Platform::get().focused_accessibility_node.peek().role();
            if matches!(role, AccessibilityRole::Menu | AccessibilityRole::MenuItem) {
                previous.request_focus();
            }
        });
        rect()
            .width(Size::fill())
            .on_key_down(move |event: Event<KeyboardEventData>| {
                let strategy = match event.key {
                    Key::Named(NamedKey::ArrowDown) => {
                        AccessibilityFocusStrategy::Forward(AccessibilityFocusMovement::InsideGroup)
                    }
                    Key::Named(NamedKey::ArrowUp) => AccessibilityFocusStrategy::Backward(
                        AccessibilityFocusMovement::InsideGroup,
                    ),
                    _ => return,
                };
                event.stop_propagation();
                event.prevent_default();
                Platform::get().send(UserEvent::FocusAccessibilityNode(strategy));
            })
            .child(self.child.clone())
    }
}
