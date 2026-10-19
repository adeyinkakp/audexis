use super::media::{MediaTarget, MediaTrack, PlaylistOccurrence, SelectedTrack};
use crate::{
    services::AppServices,
    ui::{hooks::use_playback, theme::use_audexis_colors},
};
use audexis_core::database::songs::SongRow;
use freya::{prelude::*, router::RouterContext};
use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};
#[derive(Clone, Default)]
struct Selection {
    keys: HashSet<String>,
    anchor: Option<String>,
    all: bool,
}
impl Selection {
    fn choose(&mut self, key: &str, items: &[SelectedTrack], modifiers: Modifiers) {
        self.all = false;
        let additive = modifiers.intersects(Modifiers::META | Modifiers::CONTROL);
        if modifiers.contains(Modifiers::SHIFT) {
            if let Some(anchor) = self
                .anchor
                .as_ref()
                .and_then(|key| items.iter().position(|item| item.key() == *key))
            {
                if let Some(end) = items.iter().position(|item| item.key() == key) {
                    if !additive {
                        self.keys.clear();
                    }
                    self.keys.extend(
                        items[anchor.min(end)..=anchor.max(end)]
                            .iter()
                            .map(SelectedTrack::key),
                    );
                    return;
                }
            }
        }
        if !additive {
            self.keys.clear();
        }
        if !additive || !self.keys.remove(key) {
            self.keys.insert(key.to_owned());
        }
        self.anchor = Some(key.to_owned());
    }
}
#[derive(Clone, Copy)]
struct SelectionContext {
    selection: State<Selection>,
    items: State<Arc<Vec<SelectedTrack>>>,
    modifiers: State<Modifiers>,
    select_all: State<Option<EventHandler<()>>>,
}
#[derive(PartialEq)]
pub(crate) struct TrackSelectionScope {
    pub items: Arc<Vec<SelectedTrack>>,
    pub child: Element,
    pub fill: bool,
    pub on_select_all: Option<EventHandler<()>>,
}
impl Component for TrackSelectionScope {
    fn render(&self) -> impl IntoElement {
        let mut selection = use_state(Selection::default);
        let items = use_reactive(&self.items);
        let mut modifiers = use_state(Modifiers::empty);
        let select_all = use_reactive(&self.on_select_all);
        use_provide_context(move || SelectionContext {
            selection,
            items,
            modifiers,
            select_all,
        });
        use_side_effect_with_deps(&self.items, move |items| {
            let keys = items.iter().map(SelectedTrack::key).collect::<HashSet<_>>();
            let mut state = selection.write();
            if state.all {
                state.keys = keys.clone();
            } else {
                state.keys.retain(|key| keys.contains(key));
            }
            if state.anchor.as_ref().is_some_and(|key| !keys.contains(key)) {
                state.anchor = None;
            }
        });
        rect()
            .width(Size::fill())
            .height(if self.fill { Size::fill() } else { Size::Inner })
            .on_global_key_down(move |event: Event<KeyboardEventData>| {
                modifiers.set(event.modifiers)
            })
            .on_global_key_up(move |event: Event<KeyboardEventData>| modifiers.set(event.modifiers))
            .child(self.child.clone())
    }
}
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
        let colors = use_audexis_colors();
        let key = self.item.key();
        let selected = selection.read().keys.contains(&key);
        let press_key = key.clone();
        let menu_key = key.clone();
        let id = use_a11y();
        let focus = use_focus(id);
        let mut last_click = use_state(|| None::<Instant>);
        let activate = self.on_activate.clone();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let playback = use_playback();
        let router = RouterContext::try_get();
        let target = self.target.clone();
        let playlist = self.item.playlist;
        let busy = use_state(|| false);
        rect()
            .width(self.width.clone())
            .corner_radius(6.)
            .background(if selected {
                colors.primary_washed
            } else {
                self.background
            })
            .border(
                Border::new()
                    .width(1.)
                    .alignment(BorderAlignment::Inner)
                    .fill(if focus() == Focus::Keyboard {
                        colors.primary
                    } else {
                        Color::TRANSPARENT
                    }),
            )
            .a11y_id(id)
            .a11y_focusable(true)
            .on_press(move |event: Event<PressEventData>| {
                event.stop_propagation();
                let secondary = match event.data() {
                    PressEventData::Mouse(mouse) => mouse.button.is_some_and(|button| button != MouseButton::Left),
                    _ => false,
                };
                if secondary {
                    return;
                }
                id.request_focus();
                let mods = match event.data() {
                    PressEventData::Keyboard(key) => key.modifiers,
                    _ => *modifiers.peek(),
                };
                selection.write().choose(&press_key, &items.peek(), mods);
                let modified = mods.intersects(Modifiers::META | Modifiers::CONTROL | Modifiers::SHIFT);
                let play = match event.data() {
                    PressEventData::Mouse(mouse) if mouse.button != Some(MouseButton::Right) && !modified => {
                        let now = Instant::now();
                        let double = last_click
                            .peek()
                            .is_some_and(|last| now.duration_since(last) < Duration::from_millis(350));
                        last_click.set(if double { None } else { Some(now) });
                        double
                    }
                    PressEventData::Keyboard(key) => !modified && key.key == Key::Named(NamedKey::Enter),
                    _ => {
                        last_click.set(None);
                        false
                    }
                };
                if play {
                    activate.call(());
                }
            })
            .on_key_down(move |event: Event<KeyboardEventData>| {
                if event.modifiers.intersects(Modifiers::META | Modifiers::CONTROL)
                    && matches!(&event.key, Key::Character(value) if value.eq_ignore_ascii_case("a"))
                {
                    event.stop_propagation();
                    event.prevent_default();
                    {
                        let mut state = selection.write();
                        state.keys = items.peek().iter().map(SelectedTrack::key).collect();
                        state.all = true;
                    }
                    if let Some(callback) = &*select_all.peek() {
                        callback.call(());
                    }
                } else if event.key == Key::Named(NamedKey::Escape) {
                    event.stop_propagation();
                    selection.set(Selection::default());
                }
            })
            .on_secondary_down(move |event: Event<PressEventData>| {
                event.stop_propagation();
                event.prevent_default();
                id.request_focus();
                if !selection.peek().keys.contains(&menu_key) {
                    selection.write().choose(&menu_key, &items.peek(), Modifiers::empty());
                }
                let selected = items
                    .peek()
                    .iter()
                    .filter(|item| selection.peek().keys.contains(&item.key()))
                    .cloned()
                    .collect::<Vec<_>>();
                let target = if selected.len() > 1 {
                    MediaTarget::Selection {
                        tracks: Arc::new(selected),
                    }
                } else {
                    target.clone()
                };
                crate::ui::menus::context::open(
                    &event,
                    target,
                    playback.clone(),
                    services.clone(),
                    runtime.clone(),
                    router.clone(),
                    playlist,
                    busy,
                );
            })
            .child(self.child.clone())
    }
    fn render_key(&self) -> DiffKey {
        (&("selectable-track", self.item.key())).into()
    }
}
pub(crate) fn song_items(
    tracks: &Arc<Vec<SongRow>>,
    playlist: Option<i64>,
) -> Arc<Vec<SelectedTrack>> {
    let cache = use_hook(|| {
        std::rc::Rc::new(std::cell::RefCell::new(
            None::<(Arc<Vec<SongRow>>, Option<i64>, Arc<Vec<SelectedTrack>>)>,
        ))
    });
    let mut cache = cache.borrow_mut();
    if let Some((previous, previous_playlist, items)) = cache.as_ref() {
        if Arc::ptr_eq(previous, tracks) && *previous_playlist == playlist {
            return items.clone();
        }
    }
    let items: Arc<Vec<SelectedTrack>> = Arc::new(
        tracks
            .iter()
            .map(|song| SelectedTrack {
                track: MediaTrack {
                    id: song.id,
                    path: song.path.clone(),
                },
                queue_id: None,
                playlist: playlist.zip(song.playlist_ord).map(|(playlist, ord)| {
                    PlaylistOccurrence {
                        playlist,
                        ord,
                        file: song.id,
                    }
                }),
            })
            .collect(),
    );
    *cache = Some((tracks.clone(), playlist, items.clone()));
    items
}
