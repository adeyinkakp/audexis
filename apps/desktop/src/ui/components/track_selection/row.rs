use super::*;
use std::time::Instant;

#[derive(PartialEq)]
pub(crate) struct SelectableTrack {
    pub item: SelectedTrack,
    pub target: MediaTarget,
    pub child: Element,
    pub on_activate: EventHandler<()>,
    pub background: Color,
    pub width: Size,
}
impl SelectableTrack {
    pub fn new(item: SelectedTrack, target: MediaTarget, on_activate: EventHandler<()>) -> Self {
        Self {
            item,
            target,
            on_activate,
            child: rect().into_element(),
            background: Color::TRANSPARENT,
            width: Size::fill(),
        }
    }
    pub fn width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }
    pub fn background(mut self, background: Color) -> Self {
        self.background = background;
        self
    }
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = child.into_element();
        self
    }
}
impl Component for SelectableTrack {
    fn render(&self) -> impl IntoElement {
        let context = use_consume::<SelectionContext>();
        let mut selection = context.selection;
        let items = context.items;
        let modifiers = context.modifiers;
        let select_all = context.select_all;
        let mut clicks = context.clicks;
        let colors = use_audexis_colors();
        let key = self.item.key();
        let selected = selection.read().keys.contains(&key);
        let press_key = key.clone();
        let keyboard_key = key.clone();
        let menu_key = key.clone();
        let id = use_a11y();
        let focus = use_focus(id);
        let mut pointer = use_state(|| None::<PointerPress>);
        let activate = self.on_activate.clone();
        let keyboard_activate = activate.clone();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let playback = use_playback();
        let router = RouterContext::try_get();
        let target = self.target.clone();
        let playlist = self.item.playlist;
        let busy = use_state(|| false);
        rect().width(self.width.clone()).corner_radius(6.)
            .background(if selected { colors.primary_washed } else { self.background })
            .border(Border::new().width(1.).alignment(BorderAlignment::Inner).fill(
                if focus() == Focus::Keyboard { colors.primary } else { Color::TRANSPARENT }))
            .a11y_id(id).a11y_focusable(true)
            .on_pointer_down(move |event: Event<PointerEventData>| {
                if event.data().is_primary() {
                    let point = event.global_location();
                    pointer.set(Some(PointerPress { point: (point.x, point.y), modifiers: modifiers.peek().value, dragged: false }));
                    return;
                }
                let Some(event) = event.try_map(|data| match data {
                    PointerEventData::Mouse(mouse) if mouse.button == Some(MouseButton::Right) => Some(PressEventData::Mouse(mouse)),
                    _ => None,
                }) else { return; };
                event.stop_propagation();
                event.prevent_default();
                pointer.set(None);
                clicks.write().clear();
                id.request_focus();
                if !selection.peek().keys.contains(&menu_key) {
                    selection.write().choose(&menu_key, &items.peek(), Modifiers::empty());
                    if let Some(callback) = &*select_all.peek() { callback.call(false); }
                }
                let selected = items.peek().iter().filter(|item| selection.peek().keys.contains(&item.key())).cloned().collect::<Vec<_>>();
                let target = if selected.len() > 1 { MediaTarget::Selection { tracks: Arc::new(selected) } } else { target.clone() };
                crate::ui::menus::context::open(&event, target, playback.clone(), services.clone(), runtime.clone(), router.clone(), playlist, busy);
            })
            .on_global_pointer_move(move |event: Event<PointerEventData>| {
                let Some(start) = *pointer.peek() else { return; };
                let point = event.global_location();
                if !start.dragged && model::distance(start.point, (point.x, point.y)) >= 6. {
                    pointer.set(Some(PointerPress { dragged: true, ..start }));
                    clicks.write().clear();
                }
            })
            .on_pointer_press(move |event: Event<PointerEventData>| {
                if !event.data().is_primary() { return; }
                event.stop_propagation();
                let Some(start) = pointer.take() else { return; };
                let point = event.global_location();
                if start.dragged || model::distance(start.point, (point.x, point.y)) >= 6. {
                    clicks.write().clear();
                    return;
                }
                id.request_focus();
                selection.write().choose(&press_key, &items.peek(), start.modifiers);
                if let Some(callback) = &*select_all.peek() { callback.call(false); }
                let modified = start.modifiers.intersects(Modifiers::META | Modifiers::CONTROL | Modifiers::SHIFT);
                if modified || !matches!(event.data(), PointerEventData::Mouse(_)) {
                    clicks.write().clear();
                } else if clicks.write().click(&press_key, (point.x, point.y), Instant::now()) {
                    activate.call(());
                }
            })
            .on_key_down(move |event: Event<KeyboardEventData>| {
                if event.modifiers.intersects(Modifiers::META | Modifiers::CONTROL)
                    && matches!(&event.key, Key::Character(value) if value.eq_ignore_ascii_case("a")) {
                    event.stop_propagation(); event.prevent_default();
                    let mut state = selection.write();
                    state.keys = items.peek().iter().map(SelectedTrack::key).collect();
                    state.all = true;
                    if state.anchor.is_none() { state.anchor = Some(keyboard_key.clone()); }
                    drop(state);
                    if let Some(callback) = &*select_all.peek() { callback.call(true); }
                } else if event.key == Key::Named(NamedKey::Escape) {
                    event.stop_propagation(); event.prevent_default();
                    selection.set(Selection::default());
                    if let Some(callback) = &*select_all.peek() { callback.call(false); }
                } else if event.is_press_event() {
                    event.stop_propagation(); event.prevent_default();
                    let modified = event.modifiers.intersects(Modifiers::META | Modifiers::CONTROL | Modifiers::SHIFT);
                    if event.key == Key::Named(NamedKey::Enter) && !modified {
                        keyboard_activate.call(());
                    } else {
                        selection.write().choose(&keyboard_key, &items.peek(), event.modifiers);
                        if let Some(callback) = &*select_all.peek() { callback.call(false); }
                    }
                }
            })
            .child(self.child.clone())
    }
    fn render_key(&self) -> DiffKey {
        (&("selectable-track", self.item.key())).into()
    }
}
