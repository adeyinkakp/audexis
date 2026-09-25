use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Emitter;

struct Snapshot {
    segment_id: String,
    file_id: i64,
    playlist_id: i64,
    minute: i64,
    heard_us: u64,
}
enum Message {
    Save(Snapshot),
    Barrier(crossbeam_channel::Sender<()>),
}

pub struct ListeningTime {
    sender: crossbeam_channel::Sender<Message>,
    segment_id: String,
    file_id: i64,
    playlist_id: i64,
    minute: i64,
    heard_us: u64,
    submitted_us: u64,
}
impl ListeningTime {
    pub fn start(&mut self, file_id: i64, playlist_id: Option<i64>) {
        self.flush();
        self.file_id = file_id;
        self.playlist_id = playlist_id.unwrap_or(0);
        self.reset_segment(now_minute());
    }
    fn reset_segment(&mut self, minute: i64) {
        self.segment_id = uuid::Uuid::new_v4().to_string();
        self.minute = minute;
        self.heard_us = 0;
        self.submitted_us = 0;
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            segment_id: self.segment_id.clone(),
            file_id: self.file_id,
            playlist_id: self.playlist_id,
            minute: self.minute,
            heard_us: self.heard_us,
        }
    }
    pub fn heard(&mut self, elapsed_us: u64) {
        if self.file_id <= 0 || elapsed_us == 0 {
            return;
        }
        let minute = now_minute();
        if minute != self.minute {
            if self.heard_us != self.submitted_us && !self.submit() {
                self.heard_us = self.heard_us.saturating_add(elapsed_us);
                return;
            }
            self.reset_segment(minute);
        }
        self.heard_us = self.heard_us.saturating_add(elapsed_us);
        if self.heard_us.saturating_sub(self.submitted_us) >= 5_000_000 {
            self.submit();
        }
    }
    fn submit(&mut self) -> bool {
        if self.sender.try_send(Message::Save(self.snapshot())).is_ok() {
            self.submitted_us = self.heard_us;
            true
        } else {
            false
        }
    }

    pub fn flush(&mut self) {
        if self.file_id > 0
            && self.heard_us > self.submitted_us
            && self.sender.send(Message::Save(self.snapshot())).is_ok()
        {
            self.submitted_us = self.heard_us;
        }
    }
    pub fn stop(&mut self) {
        self.flush();
        self.file_id = 0;
    }
    pub fn finish(&mut self) {
        self.stop();
        let (sender, receiver) = crossbeam_channel::bounded(1);
        if self
            .sender
            .send_timeout(Message::Barrier(sender), Duration::from_secs(2))
            .is_ok()
        {
            let _ = receiver.recv_timeout(Duration::from_secs(2));
        }
    }
}
fn now_minute() -> i64 {
    (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 60
        * 60) as i64
}

pub fn create(app: tauri::AppHandle, pool: sqlx::SqlitePool) -> ListeningTime {
    let (sender, receiver) = crossbeam_channel::bounded(128);
    std::thread::spawn(move || {
        for message in receiver {
            let snapshot = match message {
                Message::Save(snapshot) => snapshot,
                Message::Barrier(sender) => {
                    let _ = sender.send(());
                    continue;
                }
            };
            loop {
                let result: Result<(), sqlx::Error> = tauri::async_runtime::block_on(async {
                    let mut tx = pool.begin().await?;
                    let previous = sqlx::query_as::<_, (String, i64)>(
                        "SELECT segment_id,heard_us FROM listening_time_checkpoint WHERE slot=1",
                    )
                    .fetch_optional(&mut *tx)
                    .await?;
                    let total = snapshot.heard_us.min(i64::MAX as u64) as i64;
                    let already = previous
                        .filter(|(id, _)| id == &snapshot.segment_id)
                        .map_or(0, |(_, heard)| heard);
                    let delta = total.saturating_sub(already).max(0);
                    if delta > 0 {
                        sqlx::query("INSERT INTO listening_time_daily (file_id,source_playlist_id,day,heard_us) SELECT id,?,date(?,'unixepoch','localtime'),? FROM files WHERE id=? ON CONFLICT(file_id,source_playlist_id,day) DO UPDATE SET heard_us=listening_time_daily.heard_us+excluded.heard_us")
                            .bind(snapshot.playlist_id).bind(snapshot.minute).bind(delta).bind(snapshot.file_id).execute(&mut *tx).await?;
                    }
                    sqlx::query("INSERT INTO listening_time_checkpoint(slot,segment_id,heard_us) VALUES(1,?,?) ON CONFLICT(slot) DO UPDATE SET segment_id=excluded.segment_id,heard_us=excluded.heard_us")
                        .bind(&snapshot.segment_id).bind(total.max(already)).execute(&mut *tx).await?;
                    tx.commit().await?;
                    Ok(())
                });
                match result {
                    Ok(()) => {
                        let _ = app.emit("listening-time-changed", snapshot.file_id);
                        break;
                    }
                    Err(error) => {
                        tauri_plugin_log::log::error!(
                            "Could not save listening time; retrying: {error}"
                        );
                        std::thread::sleep(Duration::from_secs(2));
                    }
                }
            }
        }
    });
    ListeningTime {
        sender,
        segment_id: String::new(),
        file_id: 0,
        playlist_id: 0,
        minute: 0,
        heard_us: 0,
        submitted_us: 0,
    }
}
