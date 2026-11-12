use crate::PlaybackModes;
use tokio::sync::{broadcast, watch};

#[derive(Clone, Debug, PartialEq)]
pub struct PlaybackState {
    pub current_position: u64,
    pub duration: u64,
    pub id: i64,
    pub paused: bool,
    pub shuffled: bool,
    pub repeat_mode: crate::RepeatMode,
    pub path: Option<String>,
    pub error: Option<String>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            current_position: 0,
            duration: 0,
            id: 0,
            paused: true,
            shuffled: false,
            repeat_mode: crate::RepeatMode::Off,
            path: None,
            error: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct QueueInfo {
    pub queue_ids: Vec<String>,
    pub paths: Vec<String>,
    pub file_ids: Vec<i64>,
    pub occurrences: Vec<Option<i64>>,
    pub current_index: i32,
    pub playlist_id: Option<i64>,
}

#[derive(Clone, Debug)]
pub enum AudioEvent {
    QueueChanged(QueueInfo),
    ModesChanged(PlaybackModes),
    TrackStarted { id: i64, duration: u64 },
    TrackFinished,
    QueueFinished,
    PlayCountChanged(i64),
    ListeningHistoryChanged(i64),
    ListeningTimeChanged(i64),
    Error(String),
}

#[derive(Clone)]
pub(crate) struct EventBus {
    pub state: watch::Sender<PlaybackState>,
    pub events: broadcast::Sender<AudioEvent>,
    pub queue: watch::Sender<QueueInfo>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            state: watch::channel(PlaybackState::default()).0,
            events: broadcast::channel(128).0,
            queue: watch::channel(QueueInfo::default()).0,
        }
    }
    pub fn emit(&self, event: AudioEvent) {
        if let AudioEvent::QueueChanged(queue) = &event {
            self.queue.send_if_modified(|current| {
                if current == queue {
                    false
                } else {
                    *current = queue.clone();
                    true
                }
            });
        }
        let _ = self.events.send(event);
    }
}
