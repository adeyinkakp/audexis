use std::sync::{Arc, Mutex};
use tauri::Emitter;

struct ListenEvent {
    session: String,
    file_id: i64,
    playlist_id: Option<i64>,
    played_at: i64,
    qualified: bool,
}

pub struct PlayCounter {
    time: super::listening_time::ListeningTime,
    file_id: i64,
    session: String,
    cursor_us: u64,
    duration_us: u64,
    ranges: Vec<(u64, u64)>,
    counted: bool,
    recent_saved: bool,
    playlist_id: Option<i64>,
    started_at: i64,
    sender: crossbeam_channel::Sender<ListenEvent>,
}
impl PlayCounter {
    pub fn start(&mut self, file_id: i64, duration_ms: u64, playlist_id: Option<i64>) {
        self.time.start(file_id, playlist_id);
        self.file_id = file_id;
        self.session = uuid::Uuid::new_v4().to_string();
        self.cursor_us = 0;
        self.duration_us = duration_ms.saturating_mul(1000);
        self.ranges.clear();
        self.counted = false;
        self.recent_saved = false;
        self.playlist_id = playlist_id;
        self.started_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
    }
    pub fn flush_time(&mut self) {
        self.time.flush();
    }
    pub fn finish(&mut self) {
        self.time.finish();
    }
    pub fn stop(&mut self) {
        self.time.stop();
        self.file_id = 0;
    }
    pub fn seek(&mut self, position_ms: u64) {
        self.cursor_us = position_ms.saturating_mul(1000);
    }
    pub fn heard(&mut self, elapsed_us: u64) {
        self.time.heard(elapsed_us);
        if self.file_id <= 0 || self.counted || elapsed_us == 0 {
            return;
        }
        let start = self.cursor_us;
        self.cursor_us = self.cursor_us.saturating_add(elapsed_us);
        let end = if self.duration_us > 0 {
            self.cursor_us.min(self.duration_us)
        } else {
            self.cursor_us
        };
        if end <= start {
            return;
        }
        let mut range = (start, end);
        let mut index = 0;
        while index < self.ranges.len() {
            let old = self.ranges[index];
            if old.1 < range.0 {
                index += 1;
                continue;
            }
            if old.0 > range.1 {
                break;
            }
            range.0 = range.0.min(old.0);
            range.1 = range.1.max(old.1);
            self.ranges.remove(index);
        }
        if self.ranges.len() >= 4096 {
            return;
        }
        self.ranges.insert(index, range);
        let heard: u64 = self.ranges.iter().map(|(a, b)| b - a).sum();
        let threshold = match self.duration_us {
            0 => 240_000_000,
            duration if duration < 30_000_000 => duration.saturating_mul(9) / 10,
            duration => (duration / 2).clamp(30_000_000, 240_000_000),
        };
        let qualified = heard >= threshold;
        if qualified || (!self.recent_saved && heard >= threshold.min(5_000_000)) {
            let event = ListenEvent {
                session: self.session.clone(),
                file_id: self.file_id,
                playlist_id: self.playlist_id,
                played_at: self.started_at,
                qualified,
            };
            if self.sender.try_send(event).is_ok() {
                self.recent_saved = true;
                self.counted = qualified;
            }
        }
    }
}

pub fn create(app: tauri::AppHandle, pool: sqlx::SqlitePool) -> Arc<Mutex<PlayCounter>> {
    let time = super::listening_time::create(app.clone(), pool.clone());
    let (sender, receiver) = crossbeam_channel::bounded::<ListenEvent>(64);
    std::thread::spawn(move || {
        for event in receiver {
            let ListenEvent {
                session,
                file_id,
                playlist_id,
                played_at,
                qualified,
            } = event;

            loop {
                let result: Result<(), sqlx::Error> = tauri::async_runtime::block_on(async {
                    let mut tx = pool.begin().await?;
                    sqlx::query("INSERT OR IGNORE INTO listening_history (session_id, file_id, playlist_id, played_at) SELECT ?, id, (SELECT id FROM playlists WHERE id = ?), ? FROM files WHERE id = ?")
                        .bind(&session).bind(playlist_id).bind(played_at).bind(file_id).execute(&mut *tx).await?;
                    let inserted = if qualified {
                        sqlx::query("INSERT OR IGNORE INTO counted_plays (session_id, file_id, playlist_id, played_at) SELECT ?, id, (SELECT id FROM playlists WHERE id = ?), ? FROM files WHERE id = ?")
                        .bind(&session).bind(playlist_id).bind(played_at).bind(file_id).execute(&mut *tx).await?.rows_affected()
                    } else {
                        0
                    };
                    if inserted > 0 {
                        sqlx::query("INSERT INTO media_info (file_id, plays) VALUES (?, 1) ON CONFLICT(file_id) DO UPDATE SET plays = media_info.plays + 1")
                            .bind(file_id).execute(&mut *tx).await?;
                    }
                    tx.commit().await?;
                    Ok(())
                });
                match result {
                    Ok(()) => {
                        let _ = app.emit(
                            if qualified {
                                "play-count-changed"
                            } else {
                                "listening-history-changed"
                            },
                            file_id,
                        );
                        break;
                    }
                    Err(error) => {
                        tauri_plugin_log::log::error!(
                            "Could not save play count; retrying: {error}"
                        );
                        std::thread::sleep(std::time::Duration::from_secs(5));
                    }
                }
            }
        }
    });
    Arc::new(Mutex::new(PlayCounter {
        time,
        file_id: 0,
        session: String::new(),
        cursor_us: 0,
        duration_us: 0,
        ranges: Vec::new(),
        counted: false,
        recent_saved: false,
        playlist_id: None,
        started_at: 0,
        sender,
    }))
}
