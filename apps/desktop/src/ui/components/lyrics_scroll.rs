use freya::prelude::*;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

const FOLLOW_DURATION: Duration = Duration::from_millis(420);

#[derive(Clone, Copy)]
pub(super) struct LyricsScroll {
    pub controller: ScrollController,
    position: State<(i32, i32)>,
    task: State<Option<TaskHandle>>,
}

impl LyricsScroll {
    pub fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.cancel();
        }
    }

    pub fn follow_to(&mut self, target: i32, reduce_motion: bool) {
        self.stop();
        let target = target.min(0);
        let from = self.position.peek().1;
        if from == target {
            return;
        }
        if reduce_motion {
            self.position.set((0, target));
            return;
        }
        let mut position = self.position;
        let mut task = self.task;
        let mut ticker = RenderingTicker::get();
        let platform = Platform::get();
        task.set(Some(spawn(async move {
            let started = Instant::now();
            platform.send(UserEvent::RequestRedraw);
            loop {
                ticker.tick().await;
                let progress = started.elapsed().as_secs_f64() / FOLLOW_DURATION.as_secs_f64();
                let y = interpolate(from, target, progress);
                if position.peek().1 != y {
                    position.set((0, y));
                }
                if progress >= 1.0 {
                    break;
                }
                platform.send(UserEvent::RequestRedraw);
            }
            task.set(None);
        })));
    }
}

fn interpolate(from: i32, to: i32, progress: f64) -> i32 {
    let t = progress.clamp(0.0, 1.0);
    let eased = t * t * (3.0 - 2.0 * t);
    (from as f64 + (to as f64 - from as f64) * eased).round() as i32
}

pub(super) fn use_lyrics_scroll_controller(
    mut on_manual_scroll: impl FnMut() + 'static,
) -> LyricsScroll {
    let mut position = use_state(|| (0, 0));
    let mut task = use_state(|| None::<TaskHandle>);
    let controller = use_hook(move || {
        ScrollController::managed(
            State::create(()),
            State::create(Vec::new()),
            State::create(Callback::new(move |event| {
                let current = *position.peek();
                let next = match event {
                    ScrollEvent::X(_) => (0, current.1),
                    ScrollEvent::Y(y) => (0, y.min(0)),
                };
                if current == next {
                    return false;
                }
                on_manual_scroll();
                if let Some(running) = task.take() {
                    running.cancel();
                }
                position.set(next);
                true
            })),
            State::create(Callback::new(move |_| *position.read())),
        )
    });
    LyricsScroll {
        controller,
        position,
        task,
    }
}

pub(super) fn follow_target(
    active: Option<usize>,
    count: usize,
    heights: &HashMap<usize, f32>,
    viewport: f32,
) -> Option<i32> {
    let active = active.filter(|index| *index < count)?;
    if !viewport.is_finite() || viewport <= 0.0 {
        return None;
    }
    let mut top = 16.0;
    let mut active_top = top;
    for index in 0..count {
        let height = *heights.get(&index)?;
        if !height.is_finite() || height < 0.0 {
            return None;
        }
        if index == active {
            active_top = top;
        }
        top += height;
    }
    let max_scroll = (top + 80.0 - viewport).max(0.0);
    Some(-((active_top - viewport / 3.0).clamp(0.0, max_scroll).round() as i32))
}
