use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use ringbuf::storage::Heap;
use ringbuf::wrap::caching::Caching;
use ringbuf::SharedRb;
use ringbuf::{
    traits::{Consumer, Split},
    HeapRb,
};
use souvlaki::{MediaControlEvent, MediaControls, MediaPosition};
use std::fmt::{self, Display};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use super::queue::Queue;
use super::types::{AudioPlayerError, PartialMetadata, PlayerCmd};
use super::worker::{self, WorkerContext};

pub struct AudioPlayer {
    pub queue: Arc<Mutex<Queue>>,
    play_counter: Arc<Mutex<super::play_count::PlayCounter>>,
    stream: Option<Stream>,
    cmd_tx: crossbeam_channel::Sender<PlayerCmd>,
    consumer: Arc<Mutex<Caching<Arc<SharedRb<Heap<f32>>>, false, true>>>,
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
        app_handle: &AppHandle,
        initial_controls: MediaControls,
        pool: sqlx::SqlitePool,
    ) -> Result<Arc<Mutex<Self>>, AudioPlayerError> {
        let play_counter = super::play_count::create(app_handle.clone(), pool);
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<PlayerCmd>();
        let controls = Arc::new(Mutex::new(initial_controls));

        let cmd_tx_media = cmd_tx.clone();
        let cmd_tx_controls = cmd_tx.clone();
        let rb = HeapRb::<f32>::new(192000 * 2 * 2);
        let (producer, consumer) = rb.split();
        let queue = Arc::new(Mutex::new(Queue::new(cmd_tx.clone())));
        let consumer = Arc::new(Mutex::new(consumer));
        let database_id = Arc::new(std::sync::atomic::AtomicI64::new(0));
        let flush_output = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let audio_duration = Arc::new(AtomicU64::new(0));
        let position_ms = Arc::new(AtomicU64::new(0));

        spawn_progress_notifier(
            app_handle.clone(),
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
            controls: Arc::clone(&controls),
            app_handle: app_handle.clone(),
            database_id: Arc::clone(&database_id),
            audio_duration: Arc::clone(&audio_duration),
            flush_output: Arc::clone(&flush_output),
            paused: Arc::clone(&paused),
            position_ms: Arc::clone(&position_ms),
        };
        std::thread::spawn(move || worker::run(worker_ctx));

        let state_queue = Arc::clone(&queue);
        let position_controls = Arc::clone(&position_ms);
        let paused_controls = Arc::clone(&paused);
        let duration_controls = Arc::clone(&audio_duration);
        let db_id_controls = Arc::clone(&database_id);
        let controls_ref = Arc::clone(&controls);

        controls
            .lock()
            .unwrap()
            .attach(move |event: MediaControlEvent| match event {
                MediaControlEvent::Play => {
                    paused_controls.store(false, Ordering::Release);
                    let _ = cmd_tx_controls.send(PlayerCmd::Play { resuming: true });
                }
                MediaControlEvent::Pause => {
                    paused_controls.store(true, Ordering::Release);
                    let _ = cmd_tx_controls.send(PlayerCmd::Pause);
                }
                MediaControlEvent::Toggle => {
                    if paused_controls.load(Ordering::Acquire) {
                        paused_controls.store(false, Ordering::Release);
                        let _ = cmd_tx_controls.send(PlayerCmd::Play { resuming: true });
                    } else {
                        paused_controls.store(true, Ordering::Release);
                        let _ = cmd_tx_controls.send(PlayerCmd::Pause);
                    }
                }
                MediaControlEvent::Next => {
                    let queue = Arc::clone(&state_queue);
                    let position = Arc::clone(&position_controls);
                    let paused = Arc::clone(&paused_controls);
                    let duration = Arc::clone(&duration_controls);
                    let db_id = Arc::clone(&db_id_controls);
                    let controls = Arc::clone(&controls_ref);
                    let cmd_tx = cmd_tx_controls.clone();
                    std::thread::spawn(move || {
                        if let Err(error) = handle_media_navigation(
                            &queue, &position, &paused, &duration, &db_id, &controls, &cmd_tx, true,
                        ) {
                            tauri_plugin_log::log::error!("{error}");
                        }
                    });
                }
                MediaControlEvent::Previous => {
                    let queue = Arc::clone(&state_queue);
                    let position = Arc::clone(&position_controls);
                    let paused = Arc::clone(&paused_controls);
                    let duration = Arc::clone(&duration_controls);
                    let db_id = Arc::clone(&db_id_controls);
                    let controls = Arc::clone(&controls_ref);
                    let cmd_tx = cmd_tx_controls.clone();
                    std::thread::spawn(move || {
                        if let Err(error) = handle_media_navigation(
                            &queue, &position, &paused, &duration, &db_id, &controls, &cmd_tx,
                            false,
                        ) {
                            tauri_plugin_log::log::error!("{error}");
                        }
                    });
                }
                MediaControlEvent::Seek(direction) => {
                    let current_ms = position_controls.load(Ordering::Acquire);
                    let current_seconds = current_ms / 1000;
                    let target_seconds = match direction {
                        souvlaki::SeekDirection::Forward => current_seconds.saturating_add(10),
                        souvlaki::SeekDirection::Backward => current_seconds.saturating_sub(10),
                    };
                    let _ = cmd_tx_controls.send(PlayerCmd::Seek {
                        seconds: target_seconds,
                    });
                }
                MediaControlEvent::SetPosition(position) => {
                    let seconds = position.0.as_secs();
                    let _ = cmd_tx_controls.send(PlayerCmd::Seek { seconds });
                }
                MediaControlEvent::Stop => {
                    paused_controls.store(true, Ordering::Release);
                    position_controls.store(0, Ordering::Release);
                    duration_controls.store(0, Ordering::Release);
                    db_id_controls.store(0, Ordering::Relaxed);
                    let _ = cmd_tx_controls.send(PlayerCmd::Stop);
                    if let Err(error) = controls_ref
                        .lock()
                        .unwrap()
                        .set_playback(souvlaki::MediaPlayback::Stopped)
                    {
                        tauri_plugin_log::log::error!("Error occurred: {}", error);
                    }
                    if let Err(error) = controls_ref
                        .lock()
                        .unwrap()
                        .set_metadata(Default::default())
                    {
                        tauri_plugin_log::log::error!("Error occurred: {}", error);
                    }
                }
                _ => {
                    println!("event recieved:{:?}", event)
                }
            })
            .unwrap();

        let player = Arc::new(Mutex::new(Self {
            stream: None,
            play_counter,
            queue,
            cmd_tx,
            consumer: Arc::clone(&consumer),
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

    pub fn rebuild_active_stream(
        &mut self,
        player_arc_clone: Arc<Mutex<Self>>,
    ) -> Result<(), AudioPlayerError> {
        self.stream = None;

        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => return Err(AudioPlayerError::NoOutputDevice),
        };

        let device_config =
            device
                .default_output_config()
                .map_err(|error| AudioPlayerError::DeviceConfig {
                    message: error.to_string(),
                })?;
        let target_sample_rate = device_config.sample_rate();
        let channels = device_config.channels();

        let config = cpal::StreamConfig {
            channels,
            sample_rate: target_sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let _ = self
            .cmd_tx
            .send(PlayerCmd::UpdateDeviceConfig { target_sample_rate });

        let consumer = Arc::clone(&self.consumer);
        let flush_output = Arc::clone(&self.flush_output);
        let paused = Arc::clone(&self.paused);
        let position_ms = Arc::clone(&self.position_ms);
        let play_counter = Arc::clone(&self.play_counter);
        let output_sample_rate = target_sample_rate as u64;
        let output_channels = channels as u64;

        let stream = device
            .build_output_stream(
                config.to_owned(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if let Ok(mut c) = consumer.lock() {
                        if flush_output.swap(false, Ordering::AcqRel) {
                            c.clear();
                        }
                        if paused.load(Ordering::Acquire) {
                            data.fill(0.0);
                            return;
                        }
                        let read = c.pop_slice(data);
                        if output_channels > 0 && output_sample_rate > 0 {
                            let frames = read as u64 / output_channels;
                            let advance_ms = frames.saturating_mul(1000) / output_sample_rate;
                            position_ms.fetch_add(advance_ms, Ordering::Relaxed);
                            if let Ok(mut counter) = play_counter.try_lock() {
                                counter
                                    .heard(frames.saturating_mul(1_000_000) / output_sample_rate);
                            }
                        }
                        if read < data.len() {
                            data[read..].fill(0.0);
                            std::thread::sleep(Duration::from_millis(10));
                        }
                    } else {
                        data.fill(0.0);
                    }
                },
                move |_err| {
                    let player_inner = Arc::clone(&player_arc_clone);
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(150));
                        if let Ok(mut locked) = player_inner.lock() {
                            if let Err(error) = locked.rebuild_active_stream(player_inner.clone()) {
                                tauri_plugin_log::log::error!("{error}");
                            }
                        }
                    });
                },
                None,
            )
            .map_err(|error| AudioPlayerError::OutputStream {
                message: error.to_string(),
            })?;

        stream
            .play()
            .map_err(|error| AudioPlayerError::StartStream {
                message: error.to_string(),
            })?;
        self.stream = Some(stream);
        Ok(())
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

fn handle_media_navigation(
    queue: &Arc<Mutex<Queue>>,
    position_ms: &Arc<AtomicU64>,
    paused: &Arc<AtomicBool>,
    duration: &Arc<AtomicU64>,
    database_id: &Arc<std::sync::atomic::AtomicI64>,
    controls: &Arc<Mutex<MediaControls>>,
    cmd_tx: &crossbeam_channel::Sender<PlayerCmd>,
    next: bool,
) -> Result<(), String> {
    let next_path = {
        let mut queue = queue
            .lock()
            .map_err(|_| "Audio queue is unavailable".to_string())?;
        if next {
            queue.next()
        } else {
            queue.previous()
        }
    };

    match next_path {
        Some(_) => {
            position_ms.store(0, Ordering::Release);
            paused.store(false, Ordering::Release);
            duration.store(0, Ordering::Release);
            let _ = cmd_tx.send(PlayerCmd::Play { resuming: false });
            Ok(())
        }
        None => {
            paused.store(true, Ordering::Release);
            position_ms.store(0, Ordering::Release);
            duration.store(0, Ordering::Release);
            database_id.store(0, Ordering::Relaxed);
            let _ = cmd_tx.send(PlayerCmd::Stop);
            if let Err(error) = controls
                .lock()
                .unwrap()
                .set_playback(souvlaki::MediaPlayback::Stopped)
            {
                tauri_plugin_log::log::error!("Error occurred: {}", error);
            }
            Ok(())
        }
    }
}

fn spawn_progress_notifier(
    app_handle: AppHandle,
    controls: Arc<Mutex<MediaControls>>,
    paused: Arc<AtomicBool>,
    position_ms: Arc<AtomicU64>,
    audio_duration: Arc<AtomicU64>,
    database_id: Arc<std::sync::atomic::AtomicI64>,
) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(250));

        let is_paused = paused.load(Ordering::Acquire);
        let pos = position_ms.load(Ordering::Acquire);
        let dur = audio_duration.load(Ordering::Acquire);
        let db_id = database_id.load(Ordering::Acquire);

        let playback = if is_paused {
            souvlaki::MediaPlayback::Paused {
                progress: Some(MediaPosition(Duration::from_millis(pos))),
            }
        } else {
            souvlaki::MediaPlayback::Playing {
                progress: Some(MediaPosition(Duration::from_millis(pos))),
            }
        };

        if let Err(err) = controls.lock().unwrap().set_playback(playback) {
            tauri_plugin_log::log::error!("Error occurred: {}", err);
        }

        let _ = app_handle.emit(
            "player-update",
            super::types::NowPlayingInfo {
                current_position: pos,
                duration: dur,
                id: db_id,
                paused: is_paused,
            },
        );
    });
}
