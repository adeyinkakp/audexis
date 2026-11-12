use super::*;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

#[derive(Clone, Default, PartialEq)]
pub(super) struct Selection {
    pub keys: HashSet<String>,
    pub anchor: Option<String>,
    pub all: bool,
}
impl Selection {
    pub fn choose(&mut self, key: &str, items: &[SelectedTrack], modifiers: Modifiers) {
        if !items.iter().any(|item| item.key() == key) {
            return;
        }
        self.all = false;
        let additive = modifiers.intersects(Modifiers::META | Modifiers::CONTROL);
        if modifiers.contains(Modifiers::SHIFT)
            && let Some(anchor) = self
                .anchor
                .as_ref()
                .and_then(|key| items.iter().position(|item| item.key() == *key))
            && let Some(end) = items.iter().position(|item| item.key() == key)
        {
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
        if !additive {
            self.keys.clear();
        }
        if !additive || !self.keys.remove(key) {
            self.keys.insert(key.to_owned());
        }
        self.anchor = Some(key.to_owned());
    }
    pub fn reconcile(&mut self, items: &[SelectedTrack]) {
        let keys = items.iter().map(SelectedTrack::key).collect::<HashSet<_>>();
        if self.all {
            self.keys = keys.clone();
        } else {
            self.keys.retain(|key| keys.contains(key));
        }
        if self.anchor.as_ref().is_some_and(|key| !keys.contains(key)) {
            self.anchor = None;
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct ModifierTracker {
    pub value: Modifiers,
    pressed: HashMap<Code, Modifiers>,
}
impl ModifierTracker {
    pub fn update(&mut self, event: &KeyboardEventData, down: bool) {
        self.value = event.modifiers;
        let flag = match event.key {
            Key::Named(NamedKey::Shift) => Modifiers::SHIFT,
            Key::Named(NamedKey::Control) => Modifiers::CONTROL,
            Key::Named(NamedKey::Alt) => Modifiers::ALT,
            Key::Named(NamedKey::Meta) => Modifiers::META,
            _ => return,
        };
        if down {
            self.pressed.insert(event.code, flag);
        } else {
            self.pressed.remove(&event.code);
        }
        self.value
            .set(flag, self.pressed.values().any(|held| held.contains(flag)));
    }
}

#[derive(Clone, Default)]
pub(super) struct ClickTracker(Option<(String, Instant, (f64, f64))>);
impl ClickTracker {
    pub fn clear(&mut self) {
        self.0 = None;
    }
    pub fn click(&mut self, key: &str, point: (f64, f64), now: Instant) -> bool {
        let double = self.0.as_ref().is_some_and(|(last_key, at, last_point)| {
            last_key == key
                && now.duration_since(*at) <= Duration::from_millis(500)
                && distance(*last_point, point) < 6.
        });
        self.0 = if double {
            None
        } else {
            Some((key.to_owned(), now, point))
        };
        double
    }
}
pub(super) fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

#[derive(Clone, Copy)]
pub(super) struct PointerPress {
    pub point: (f64, f64),
    pub modifiers: Modifiers,
    pub dragged: bool,
}
