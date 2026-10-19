use freya::prelude::*;
use std::time::Instant;

pub(super) fn use_drag_offset(target: f32, reduced: bool, dragging: bool) -> f32 {
    let mut position = use_state(|| 0.0f32);
    let mut running = use_state(|| None::<TaskHandle>);
    use_side_effect_with_deps(
        &(target, reduced, dragging),
        move |&(target, reduced, dragging)| {
            if let Some(task) = running.take() {
                task.cancel();
            }
            let from = *position.peek();
            if reduced || !dragging || (from - target).abs() < 0.1 {
                position.set_if_modified(target);
                return;
            }
            let mut ticker = RenderingTicker::get();
            let platform = Platform::get();
            running.set(Some(spawn(async move {
                let started = Instant::now();
                platform.send(UserEvent::RequestRedraw);
                loop {
                    ticker.tick().await;
                    let t = (started.elapsed().as_secs_f32() / 0.16).min(1.0);
                    let value = from + (target - from) * (1.0 - (1.0 - t).powi(3));
                    position.set_if_modified(value);
                    if t >= 1.0 {
                        break;
                    }
                    platform.send(UserEvent::RequestRedraw);
                }
                running.set(None);
            })));
        },
    );
    if dragging && !reduced {
        *position.read()
    } else {
        target
    }
}
