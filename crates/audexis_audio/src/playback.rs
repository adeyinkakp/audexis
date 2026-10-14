use crate::events::{AudioEvent, EventBus, PlaybackState};
use crate::media::MediaSession;
use cpal::Stream;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::SharedRb;
use ringbuf::storage::Heap;
use ringbuf::wrap::caching::Caching;
use ringbuf::{
    HeapRb,
    traits::{Consumer, Observer, Split},
};
use souvlaki::{MediaControlEvent, MediaControls, MediaPosition};
use std::fmt::{self, Display};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use super::progress::FrameProgress;
use super::queue::Queue;
use super::types::{AudioPlayerError, PartialMetadata, PlayerCmd};
use super::worker::{self, WorkerContext};
type ConsumerType = Arc<Mutex<Caching<Arc<SharedRb<Heap<f32>>>, false, true>>>;

pub struct AudioPlayer {
    events: EventBus,
    shutdown: Arc<AtomicBool>,
    controls: Arc<Mutex<MediaSession>>,
    pub equalizer: Arc<Mutex<super::equalizer::Settings>>,
    pub queue: Arc<Mutex<Queue>>,
    play_counter: Arc<Mutex<super::play_count::PlayCounter>>,
    stream: Option<Stream>,
    cmd_tx: crossbeam_channel::Sender<PlayerCmd>,
    consumer: ConsumerType,
    consumed_samples: Arc<AtomicU64>,
    heard_us: Arc<AtomicU64>,
    flush_output: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    position_ms: Arc<AtomicU64>,
}

impl Display for AudioPlayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stream player")
    }
}

impl AudioPlayer {
    pub fn new(
        initial_controls: Option<MediaControls>,
        pool: sqlx::SqlitePool,
        runtime: tokio::runtime::Handle,
        equalizer: super::equalizer::Settings,
    ) -> Result<Arc<Mutex<Self>>, AudioPlayerError> {
        let events = EventBus::new();
        let shutdown = Arc::new(AtomicBool::new(false));
        let play_counter = super::play_count::create(events.clone(), pool, runtime);
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<PlayerCmd>();
        let controls = Arc::new(Mutex::new(MediaSession(initial_controls)));

        let cmd_tx_media = cmd_tx.clone();
        let cmd_tx_controls = cmd_tx.clone();
        let device = cpal::default_host()
            .default_output_device()
            .ok_or(AudioPlayerError::NoOutputDevice)?;
        let output =
            device
                .default_output_config()
                .map_err(|error| AudioPlayerError::DeviceConfig {
                    message: error.to_string(),
                })?;
        let rb = HeapRb::<f32>::new(worker::output_buffer_samples(
            output.sample_rate(),
            output.channels() as usize,
        ));
        let (producer, consumer) = rb.split();
        let queue = Arc::new(Mutex::new(Queue::new(cmd_tx.clone())));
        let consumer = Arc::new(Mutex::new(consumer));
        let database_id = Arc::new(std::sync::atomic::AtomicI64::new(0));
        let flush_output = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(true));
        let audio_duration = Arc::new(AtomicU64::new(0));
        let position_ms = Arc::new(AtomicU64::new(0));
        let consumed_samples = Arc::new(AtomicU64::new(0));
        let heard_us = Arc::new(AtomicU64::new(0));

        spawn_progress_notifier(
            events.clone(),
            Arc::clone(&shutdown),
            Arc::clone(&controls),
            Arc::clone(&paused),
            Arc::clone(&position_ms),
            Arc::clone(&audio_duration),
            Arc::clone(&database_id),
        );

        let worker_ctx = WorkerContext {
            play_counter: Arc::clone(&play_counter),
            queue: Arc::clone(&queue),
            cmd_rx,
            cmd_tx: cmd_tx_media,
            producer,
            consumed_samples: Arc::clone(&consumed_samples),
            produced_samples: 0,
            heard_us: heard_us.clone(),
            controls: Arc::clone(&controls),
            events: events.clone(),
            shutdown: Arc::clone(&shutdown),
            database_id: Arc::clone(&database_id),
            audio_duration: Arc::clone(&audio_duration),
            flush_output: Arc::clone(&flush_output),
            paused: Arc::clone(&paused),
            position_ms: Arc::clone(&position_ms),
        };
        std::thread::spawn(move || worker::run(worker_ctx));

        let position_controls = Arc::clone(&position_ms);
        let paused_controls = Arc::clone(&paused);
        if let Err(error) = controls.lock().unwrap().attach(move |event| {
            let command = match event {
                MediaControlEvent::Play => PlayerCmd::Play { resuming: true },
                MediaControlEvent::Pause => PlayerCmd::Pause,
                MediaControlEvent::Toggle => {
                    if paused_controls.load(Ordering::Acquire) {
                        PlayerCmd::Play { resuming: true }
                    } else {
                        PlayerCmd::Pause
                    }
                }
                MediaControlEvent::Next => PlayerCmd::Next,
                MediaControlEvent::Previous => PlayerCmd::Previous,
                MediaControlEvent::Stop => PlayerCmd::Stop,
                MediaControlEvent::Seek(direction) => {
                    let position = position_controls.load(Ordering::Acquire);
                    PlayerCmd::SeekMilliseconds {
                        milliseconds: match direction {
                            souvlaki::SeekDirection::Forward => position.saturating_add(10_000),
                            souvlaki::SeekDirection::Backward => position.saturating_sub(10_000),
                        },
                    }
                }
                MediaControlEvent::SetPosition(position) => PlayerCmd::SeekMilliseconds {
                    milliseconds: position.0.as_millis().min(u128::from(u64::MAX)) as u64,
                },
                _ => return,
            };
            let _ = cmd_tx_controls.send(command);
        }) {
            tracing::warn!(%error, "OS media controls unavailable");
        }

        let player = Arc::new(Mutex::new(Self {
            events,
            shutdown,
            controls,
            equalizer: Arc::new(Mutex::new(equalizer)),
            stream: None,
            play_counter,
            queue,
            cmd_tx,
            consumer: Arc::clone(&consumer),
            consumed_samples,
            heard_us,
            flush_output: Arc::clone(&flush_output),
            paused: Arc::clone(&paused),
            position_ms: Arc::clone(&position_ms),
        }));

        {
            let mut locked = player.lock().unwrap();
            locked.rebuild_active_stream(Arc::clone(&player))?;
        }

        Ok(player)
    }

    pub fn update_media_controls(&self, metadata: PartialMetadata) {
        let _ = self
            .cmd_tx
            .send(PlayerCmd::UpdateControlsMetadata { metadata });
    }

    pub fn restart(&self) {
        self.position_ms.store(0, Ordering::Release);
        self.paused.store(false, Ordering::Release);
        self.flush_output.store(true, Ordering::Release);
        let _ = self.cmd_tx.send(PlayerCmd::Play { resuming: false });
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::Release);
        let _ = self.cmd_tx.send(PlayerCmd::Pause);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::Release);
        let _ = self.cmd_tx.send(PlayerCmd::Play { resuming: true });
    }

    pub fn stop(&self) {
        self.position_ms.store(0, Ordering::Release);
        self.paused.store(true, Ordering::Release);
        self.flush_output.store(true, Ordering::Release);
        let _ = self.cmd_tx.send(PlayerCmd::Stop);
    }

    pub fn seek(&self, seconds: u64) {
        self.flush_output.store(true, Ordering::Release);
        let _ = self.cmd_tx.send(PlayerCmd::Seek { seconds });
    }

    pub fn seek_milliseconds(&self, milliseconds: u64) {
        self.flush_output.store(true, Ordering::Release);
        let _ = self
            .cmd_tx
            .send(PlayerCmd::SeekMilliseconds { milliseconds });
    }

    pub fn finish_listening(&self) {
        self.paused.store(true, Ordering::Release);
        if let Ok(mut counter) = self.play_counter.lock() {
            counter.finish();
        }
    }

    pub fn position_ms(&self) -> u64 {
        self.position_ms.load(Ordering::Acquire)
    }
}

#[derive(Clone)]
pub struct PlayerHandle {
    commands: crossbeam_channel::Sender<PlayerCmd>,
    events: EventBus,
}
impl PlayerHandle {
    pub fn send(&self, command: PlayerCmd) -> Result<(), String> {
        self.commands.try_send(command).map_err(|e| e.to_string())
    }
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<PlaybackState> {
        self.events.state.subscribe()
    }
    pub fn subscribe_queue(&self) -> tokio::sync::watch::Receiver<crate::QueueInfo> {
        self.events.queue.subscribe()
    }
    pub fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<AudioEvent> {
        self.events.events.subscribe()
    }
}
impl AudioPlayer {
    pub fn handle(&self) -> PlayerHandle {
        PlayerHandle {
            commands: self.cmd_tx.clone(),
            events: self.events.clone(),
        }
    }
}
impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        self.stream = None;
        if let Ok(mut controls) = self.controls.lock() {
            controls.detach();
        }
        if let Ok(mut counter) = self.play_counter.lock() {
            counter.finish();
        }
    }
}

mod stream;

mod progress;
use progress::spawn_progress_notifier;
