use freya::prelude::*;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Gesture {
    axis: Option<bool>,
    last: Option<Instant>,
}
#[derive(Clone)]
pub struct TableScroll {
    pub controller: ScrollController,
    pub position: State<(i32, i32)>,
    pub viewport: State<(f32, f32)>,
    gesture: Rc<RefCell<Gesture>>,
    shift: State<bool>,
}
impl TableScroll {
    pub fn wheel(&self, event: &Event<WheelEventData>) {
        let (x, y) = self.deltas(event);
        if !x.is_finite() || !y.is_finite() || (x == 0.0 && y == 0.0) {
            return;
        }
        let mut gesture = self.gesture.borrow_mut();
        gesture.axis = Some(x.abs() > y.abs());
        gesture.last = Some(Instant::now());
    }
    pub fn deltas(&self, event: &Event<WheelEventData>) -> (f64, f64) {
        if *self.shift.peek() && event.source == WheelSource::Device {
            (event.delta_y, event.delta_x)
        } else {
            (event.delta_x, event.delta_y)
        }
    }
    pub fn key(&mut self, event: &Event<KeyboardEventData>, down: bool) {
        if down {
            self.release_axis();
        }
        if event.key == Key::Named(NamedKey::Shift) {
            self.shift.set(down);
        }
    }
    pub fn release_axis(&self) {
        *self.gesture.borrow_mut() = Gesture::default();
    }
}
pub fn use_table_scroll(width: f32, height: f32) -> TableScroll {
    let mut position = use_state(|| (0, 0));
    let viewport = use_state(|| (0.0f32, 0.0f32));
    let bounds = use_reactive(&(width, height));
    let gesture = use_hook(|| Rc::new(RefCell::new(Gesture::default())));
    let wheel_gesture = gesture.clone();
    let shift = use_state(|| false);
    let controller = use_hook(move || {
        ScrollController::managed(
            State::create(()),
            State::create(Vec::new()),
            State::create(Callback::new(move |event| {
                let current = *position.peek();
                let wheel = wheel_gesture.borrow();
                let axis = wheel.axis.filter(|_| {
                    wheel
                        .last
                        .is_some_and(|last| last.elapsed() < Duration::from_millis(180))
                });
                let dimensions = *bounds.peek();
                let visible = *viewport.peek();
                let next = match event {
                    ScrollEvent::X(_) if axis == Some(false) => return false,
                    ScrollEvent::Y(_) if axis == Some(true) => return false,
                    ScrollEvent::X(x) => (
                        x.clamp(-((dimensions.0 - visible.0).max(0.0).ceil() as i32), 0),
                        current.1,
                    ),
                    ScrollEvent::Y(y) => (
                        current.0,
                        y.clamp(-((dimensions.1 - visible.1).max(0.0).ceil() as i32), 0),
                    ),
                };
                if next == current {
                    return false;
                }
                position.set(next);
                true
            })),
            State::create(Callback::new(move |_| *position.read())),
        )
    });
    use_side_effect(move || {
        let (w, h) = *bounds.read();
        let (vw, vh) = *viewport.read();
        let (x, y) = *position.peek();
        let next = (
            x.clamp(-((w - vw).max(0.0).ceil() as i32), 0),
            y.clamp(-((h - vh).max(0.0).ceil() as i32), 0),
        );
        if (x, y) != next {
            position.set(next);
        }
    });
    TableScroll {
        controller,
        position,
        viewport,
        gesture,
        shift,
    }
}
