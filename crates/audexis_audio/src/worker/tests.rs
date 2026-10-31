use super::*;
use crate::{PlayerCmd, queue_track::UnresolvedTrack};
use ringbuf::{HeapRb, traits::Split};

struct Wav(std::path::PathBuf);
impl Wav {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("audexis-{}.wav", uuid::Uuid::new_v4()));
        let rate = 44100u32;
        let frames = rate;
        let bytes = frames * 2;
        let mut wav = Vec::new();
        wav.extend(b"RIFF");
        wav.extend((36 + bytes).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16u32.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(1u16.to_le_bytes());
        wav.extend(rate.to_le_bytes());
        wav.extend((rate * 2).to_le_bytes());
        wav.extend(2u16.to_le_bytes());
        wav.extend(16u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend(bytes.to_le_bytes());
        for frame in 0..frames {
            let sample = ((frame as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin()
                * 8000.0) as i16;
            wav.extend(sample.to_le_bytes());
        }
        std::fs::write(&path, wav).unwrap();
        Self(path)
    }
}
impl Drop for Wav {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[tokio::test]
async fn commands_publish_state_and_decoder_errors_without_a_device() {
    let wav = Wav::new();
    let events = EventBus::new();
    let mut snapshots = events.state.subscribe();
    let mut queue_updates = events.queue.subscribe();
    let (tx, rx) = crossbeam_channel::unbounded();
    let (producer, _consumer) = HeapRb::<f32>::new(256).split();
    let shutdown = Arc::new(AtomicBool::new(false));
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_lazy("sqlite::memory:")
        .unwrap();
    let ctx = WorkerContext {
        queue: Arc::new(Mutex::new(Queue::new(tx.clone()))),
        play_counter: crate::play_count::create(
            events.clone(),
            pool,
            tokio::runtime::Handle::current(),
        ),
        cmd_rx: rx,
        cmd_tx: tx.clone(),
        producer,
        controls: Arc::new(Mutex::new(MediaSession(None))),
        events,
        shutdown: shutdown.clone(),
        database_id: Arc::new(AtomicI64::new(0)),
        audio_duration: Arc::new(AtomicU64::new(0)),
        flush_output: Arc::new(AtomicBool::new(false)),
        paused: Arc::new(AtomicBool::new(true)),
        position_ms: Arc::new(AtomicU64::new(0)),
        consumed_samples: Arc::new(AtomicU64::new(0)),
        produced_samples: 0,
        heard_us: Arc::new(AtomicU64::new(0)),
    };
    let worker = std::thread::spawn(move || run(ctx));
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        tx.send(PlayerCmd::Load {
            tracks: vec![UnresolvedTrack {
                id: 0,
                path: wav.0.to_string_lossy().into_owned(),
                occurrence: None,
            }],
        })
        .unwrap();
        loop {
            snapshots.changed().await.unwrap();
            let state = snapshots.borrow_and_update().clone();
            if state.path.is_some() && !state.paused {
                assert_eq!(state.duration, 1000);
                break;
            }
        }
        tx.send(PlayerCmd::ToggleShuffle).unwrap();
        loop {
            snapshots.changed().await.unwrap();
            if snapshots.borrow_and_update().shuffled {
                break;
            }
        }
        for mode in [
            crate::RepeatMode::Queue,
            crate::RepeatMode::Track,
            crate::RepeatMode::Off,
        ] {
            tx.send(PlayerCmd::CycleRepeat).unwrap();
            loop {
                snapshots.changed().await.unwrap();
                if snapshots.borrow_and_update().repeat_mode == mode {
                    break;
                }
            }
        }
        tx.send(PlayerCmd::Pause).unwrap();
        loop {
            snapshots.changed().await.unwrap();
            if snapshots.borrow_and_update().paused {
                break;
            }
        }
        tx.send(PlayerCmd::SeekMilliseconds { milliseconds: 500 })
            .unwrap();
        loop {
            snapshots.changed().await.unwrap();
            if snapshots.borrow_and_update().current_position == 500 {
                break;
            }
        }
        tx.send(PlayerCmd::Stop).unwrap();
        loop {
            snapshots.changed().await.unwrap();
            if snapshots.borrow_and_update().path.is_none() {
                break;
            }
        }

        tx.send(PlayerCmd::Load {
            tracks: [0, 1]
                .into_iter()
                .map(|id| UnresolvedTrack {
                    id,
                    path: wav.0.to_string_lossy().into_owned(),
                    occurrence: None,
                })
                .collect(),
        })
        .unwrap();
        loop {
            snapshots.changed().await.unwrap();
            if snapshots.borrow_and_update().path.is_some() {
                break;
            }
        }
        tx.send(PlayerCmd::Pause).unwrap();
        tx.send(PlayerCmd::RemoveMissingFiles { file_ids: vec![0] })
            .unwrap();
        loop {
            snapshots.changed().await.unwrap();
            let snapshot = snapshots.borrow_and_update().clone();
            if snapshot.id == 1 && snapshot.paused {
                break;
            }
        }
        tx.send(PlayerCmd::RemoveMissingFiles { file_ids: vec![1] })
            .unwrap();
        loop {
            snapshots.changed().await.unwrap();
            let snapshot = snapshots.borrow_and_update().clone();
            if snapshot.path.is_none() {
                assert!(snapshot.paused);
                break;
            }
        }
        tx.send(PlayerCmd::LoadAt {
            index: 1,
            tracks: (0..3)
                .map(|_| UnresolvedTrack {
                    id: 0,
                    path: wav.0.to_string_lossy().into_owned(),
                    occurrence: None,
                })
                .collect(),
        })
        .unwrap();
        let ids = loop {
            queue_updates.changed().await.unwrap();
            let q = queue_updates.borrow_and_update().clone();
            if q.queue_ids.len() == 3 {
                assert_eq!(q.current_index, 1);
                break q.queue_ids;
            }
        };
        tx.send(PlayerCmd::Previous).unwrap();
        loop {
            queue_updates.changed().await.unwrap();
            let q = queue_updates.borrow_and_update();
            if q.current_index == 0 {
                assert_eq!(q.queue_ids, ids);
                break;
            }
        }
        tx.send(PlayerCmd::Next).unwrap();
        loop {
            queue_updates.changed().await.unwrap();
            let q = queue_updates.borrow_and_update();
            if q.current_index == 1 {
                assert_eq!(q.queue_ids, ids);
                break;
            }
        }
        tx.send(PlayerCmd::Previous).unwrap();
        loop {
            queue_updates.changed().await.unwrap();
            if queue_updates.borrow_and_update().current_index == 0 {
                break;
            }
        }
        tx.send(PlayerCmd::ReorderQueueEntry {
            queue_id: ids[2].clone(),
            target_id: ids[0].clone(),
        })
        .unwrap();
        loop {
            queue_updates.changed().await.unwrap();
            if queue_updates.borrow_and_update().queue_ids[0] == ids[2] {
                break;
            }
        }
        tx.send(PlayerCmd::PlayQueueEntry {
            queue_id: ids[1].clone(),
        })
        .unwrap();
        loop {
            queue_updates.changed().await.unwrap();
            let q = queue_updates.borrow_and_update();
            if q.queue_ids[q.current_index as usize] == ids[1] {
                break;
            }
        }
        tx.send(PlayerCmd::RemoveQueueEntry {
            queue_id: ids[0].clone(),
        })
        .unwrap();
        loop {
            queue_updates.changed().await.unwrap();
            let q = queue_updates.borrow_and_update();
            if q.queue_ids.len() == 2 {
                assert_eq!(q.queue_ids[q.current_index as usize], ids[1]);
                break;
            }
        }
        tx.send(PlayerCmd::Pause).unwrap();
        tx.send(PlayerCmd::Enqueue {
            tracks: [42, 43]
                .into_iter()
                .map(|id| UnresolvedTrack {
                    id,
                    path: wav.0.to_string_lossy().into_owned(),
                    occurrence: None,
                })
                .collect(),
            next: true,
        })
        .unwrap();
        loop {
            queue_updates.changed().await.unwrap();
            let q = queue_updates.borrow_and_update();
            if q.queue_ids.len() == 4 {
                let current = q.current_index as usize;
                assert_eq!(q.queue_ids[current], ids[1]);
                assert_eq!(&q.file_ids[current + 1..current + 3], &[42, 43]);
                break;
            }
        }
        tx.send(PlayerCmd::Enqueue {
            tracks: vec![UnresolvedTrack {
                id: 44,
                path: wav.0.to_string_lossy().into_owned(),
                occurrence: None,
            }],
            next: false,
        })
        .unwrap();
        loop {
            queue_updates.changed().await.unwrap();
            let q = queue_updates.borrow_and_update();
            if q.queue_ids.len() == 5 {
                assert_eq!(q.file_ids.last(), Some(&44));
                assert_eq!(q.queue_ids[q.current_index as usize], ids[1]);
                break;
            }
        }
        tx.send(PlayerCmd::ClearQueue).unwrap();
        loop {
            queue_updates.changed().await.unwrap();
            if queue_updates.borrow_and_update().queue_ids.is_empty() {
                break;
            }
        }
        tx.send(PlayerCmd::Load {
            tracks: vec![UnresolvedTrack {
                id: 0,
                path: wav
                    .0
                    .with_extension("missing")
                    .to_string_lossy()
                    .into_owned(),
                occurrence: None,
            }],
        })
        .unwrap();
        loop {
            snapshots.changed().await.unwrap();
            let state = snapshots.borrow_and_update().clone();
            if state.error.is_some() {
                assert!(state.paused);
                assert!(state.path.is_none());
                break;
            }
        }
    })
    .await;
    shutdown.store(true, Ordering::Release);
    worker.join().unwrap();
    result.expect("audio worker did not respond");
}
