use super::*;

#[derive(PartialEq)]
pub(crate) struct TrackSelectionScope {
    pub items: Arc<Vec<SelectedTrack>>,
    pub child: Element,
    pub fill: bool,
    pub on_select_all: Option<EventHandler<bool>>,
}
impl Component for TrackSelectionScope {
    fn render(&self) -> impl IntoElement {
        let mut selection = use_state(Selection::default);
        let items = use_reactive(&self.items);
        let mut modifiers = use_state(ModifierTracker::default);
        let mut clicks = use_state(ClickTracker::default);
        let select_all = use_reactive(&self.on_select_all);
        use_provide_context(move || SelectionContext {
            selection,
            items,
            modifiers,
            select_all,
            clicks,
        });
        use_side_effect_with_deps(&self.items, move |items| {
            let mut next = selection.peek().clone();
            next.reconcile(items);
            selection.set_if_modified(next);
        });
        let focused = *Platform::get().is_app_focused.read();
        use_side_effect_with_deps(&focused, move |_| {
            modifiers.set(ModifierTracker::default());
            clicks.write().clear();
        });
        rect()
            .width(Size::fill())
            .height(if self.fill { Size::fill() } else { Size::Inner })
            .on_global_key_down(move |event: Event<KeyboardEventData>| {
                modifiers.write().update(&event, true);
                clicks.write().clear();
            })
            .on_global_key_up(move |event: Event<KeyboardEventData>| {
                modifiers.write().update(&event, false)
            })
            .child(self.child.clone())
    }
}
