use crate::services::AppServices;
use freya::prelude::*;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

const DISTANCE_SCALE: f64 = 20.0 / 53.0;
const EASE_TIME: Duration = Duration::from_millis(120);

fn is_wheel_step(x: f64, y: f64) -> bool {
    let step =
        |v: f64| v == 0.0 || (v.abs() >= 52.99 && (v / 53.0 - (v / 53.0).round()).abs() < 0.001);
    (x != 0.0 || y != 0.0) && step(x) && step(y)
}
fn ease(from: f64, to: f64, progress: f64) -> f64 {
    let t = progress.clamp(0.0, 1.0);
    from + (to - from) * (1.0 - (1.0 - t).powi(3))
}
#[derive(Default)]
struct Motion {
    target: f64,
    bounds: f64,
    suppress_native: [bool; 2],
}
fn destination(target: f64, delta: f64, bounds: f64) -> f64 {
    (target + delta * DISTANCE_SCALE).clamp(-bounds, 0.0)
}

#[derive(PartialEq)]
pub struct SmoothScroll {
    children: Vec<Element>,
    horizontal: bool,
    spacing: f32,
}
impl SmoothScroll {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            horizontal: false,
            spacing: 0.0,
        }
    }
    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}
impl ChildrenExt for SmoothScroll {
    fn get_children(&mut self) -> &mut Vec<Element> {
        &mut self.children
    }
}
impl Component for SmoothScroll {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Arc<AppServices>>();
        let reduce_motion = services
            .user_config
            .lock()
            .map(|c| c.reduce_motion)
            .unwrap_or(true);
        let axis = if self.horizontal { 0 } else { 1 };
        let motion = use_hook(|| Rc::new(RefCell::new(Motion::default())));
        let mut position = use_state(|| 0i32);
        let mut task = use_state(|| None::<TaskHandle>);
        let mut viewport = use_state(|| 0.0f32);
        let mut content = use_state(|| 0.0f32);
        let mut shift = use_state(|| false);
        let bounds_motion = motion.clone();
        use_side_effect(move || {
            let bounds = (*content.read() - *viewport.read()).max(0.0) as f64;
            let mut motion = bounds_motion.borrow_mut();
            motion.bounds = bounds;
            if motion.target < -bounds {
                if let Some(running) = task.take() {
                    running.cancel();
                }
                motion.target = -bounds;
                position.set((-bounds).round() as i32);
            }
        });
        let direct_motion = motion.clone();
        let controller = use_hook(move || {
            let on_scroll = State::create(Callback::new(move |event| {
                let (requested_axis, requested) = match event {
                    ScrollEvent::X(x) => (0, x),
                    ScrollEvent::Y(y) => (1, y),
                };
                let suppressed =
                    std::mem::take(&mut direct_motion.borrow_mut().suppress_native[requested_axis]);
                if suppressed || requested_axis != axis {
                    return false;
                }
                if requested == *position.peek() {
                    return false;
                }
                if let Some(running) = task.take() {
                    running.cancel();
                }
                direct_motion.borrow_mut().target = requested as f64;
                position.set(requested);
                true
            }));
            ScrollController::managed(
                State::create(()),
                State::create(Vec::new()),
                on_scroll,
                State::create(Callback::new(move |_| {
                    if axis == 0 {
                        (*position.read(), 0)
                    } else {
                        (0, *position.read())
                    }
                })),
            )
        });
        let wheel_motion = motion.clone();
        let pointer_motion = motion.clone();
        let key_motion = motion.clone();
        let direction = if self.horizontal {
            Direction::Horizontal
        } else {
            Direction::Vertical
        };
        super::VisibilityViewport::new().child(
            rect()
                .expanded()
                .on_sized(move |event: Event<SizedEventData>| {
                    viewport.set_if_modified(if axis == 0 {
                        event.area.width()
                    } else {
                        event.area.height()
                    })
                })
                .on_pointer_down(move |_| {
                    if let Some(running) = task.take() {
                        running.cancel();
                    }
                    let mut motion = pointer_motion.borrow_mut();
                    motion.target = *position.peek() as f64;
                    motion.suppress_native = [false; 2];
                })
                .on_global_key_down(move |event: Event<KeyboardEventData>| {
                    if event.key == Key::Named(NamedKey::Shift) {
                        shift.set(true);
                    }
                })
                .on_global_key_up(move |event: Event<KeyboardEventData>| {
                    if event.key == Key::Named(NamedKey::Shift) {
                        shift.set(false);
                    }
                })
                .child(
                    ScrollView::new_controlled(controller)
                        .direction(direction)
                        .child(
                            rect()
                                .direction(direction)
                                .width(if self.horizontal {
                                    Size::Inner
                                } else {
                                    Size::fill()
                                })
                                .spacing(self.spacing)
                                .on_sized(move |event: Event<SizedEventData>| {
                                    content.set_if_modified(if axis == 0 {
                                        event.area.width()
                                    } else {
                                        event.area.height()
                                    })
                                })
                                .on_wheel(move |event: Event<WheelEventData>| {
                                    let (x, y) =
                                        if *shift.peek() && event.source == WheelSource::Device {
                                            (event.delta_y, event.delta_x)
                                        } else {
                                            (event.delta_x, event.delta_y)
                                        };
                                    if !x.is_finite() || !y.is_finite() || (x == 0.0 && y == 0.0) {
                                        return;
                                    }
                                    let selected = if x.abs() > y.abs() { 0 } else { 1 };
                                    if selected != axis {
                                        wheel_motion.borrow_mut().suppress_native = [true; 2];
                                        return;
                                    }

                                    event.stop_propagation();
                                    if let Some(running) = task.take() {
                                        running.cancel();
                                    }
                                    let current = *position.peek();
                                    let target = {
                                        let mut motion = wheel_motion.borrow_mut();
                                        motion.suppress_native = [false; 2];
                                        motion.target = destination(
                                            motion.target,
                                            if axis == 0 { x } else { y },
                                            motion.bounds,
                                        );
                                        motion.target
                                    };
                                    let smooth = !reduce_motion
                                        && event.source == WheelSource::Device
                                        && is_wheel_step(x, y);
                                    if !smooth {
                                        position.set(target.round() as i32);
                                        return;
                                    }
                                    let mut ticker = RenderingTicker::get();
                                    let platform = Platform::get();
                                    task.set(Some(spawn(async move {
                                        let start = Instant::now();
                                        platform.send(UserEvent::RequestRedraw);
                                        loop {
                                            ticker.tick().await;
                                            let progress = start.elapsed().as_secs_f64()
                                                / EASE_TIME.as_secs_f64();
                                            position.set(
                                                ease(current as f64, target, progress).round()
                                                    as i32,
                                            );
                                            if progress >= 1.0 {
                                                break;
                                            }
                                            platform.send(UserEvent::RequestRedraw);
                                        }
                                        task.set(None);
                                    })));
                                })
                                .on_key_down(move |_| {
                                    if let Some(running) = task.take() {
                                        running.cancel();
                                    }
                                    let mut motion = key_motion.borrow_mut();
                                    motion.target = *position.peek() as f64;
                                    motion.suppress_native = [false; 2];
                                })
                                .children(self.children.clone()),
                        ),
                ),
        )
    }
}
