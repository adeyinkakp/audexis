use super::*;

pub(super) fn spawn_progress_notifier(
    events: EventBus,
    shutdown: Arc<AtomicBool>,
    controls: Arc<Mutex<MediaSession>>,
    paused: Arc<AtomicBool>,
    position_ms: Arc<AtomicU64>,
    audio_duration: Arc<AtomicU64>,
    database_id: Arc<std::sync::atomic::AtomicI64>,
) {
    std::thread::spawn(move || {
        while !shutdown.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(250));

            let is_paused = paused.load(Ordering::Acquire);
            let pos = position_ms.load(Ordering::Acquire);
            let dur = audio_duration.load(Ordering::Acquire);
            let db_id = database_id.load(Ordering::Acquire);

            let path = events.state.borrow().path.clone();
            let playback = if path.is_none() {
                souvlaki::MediaPlayback::Stopped
            } else if is_paused {
                souvlaki::MediaPlayback::Paused {
                    progress: Some(MediaPosition(Duration::from_millis(pos))),
                }
            } else {
                souvlaki::MediaPlayback::Playing {
                    progress: Some(MediaPosition(Duration::from_millis(pos))),
                }
            };

            if let Err(err) = controls.lock().unwrap().set_playback(playback) {
                tracing::error!("Error occurred: {}", err);
            }

            events.state.send_if_modified(|state| {
                let next = PlaybackState {
                    current_position: pos,
                    duration: dur,
                    id: db_id,
                    paused: is_paused,
                    shuffled: state.shuffled,
                    repeat_mode: state.repeat_mode,
                    path: state.path.clone(),
                    error: state.error.clone(),
                };
                if *state == next {
                    return false;
                }
                *state = next;
                true
            });
        }
    });
}
