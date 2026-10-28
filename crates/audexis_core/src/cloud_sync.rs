mod client;
mod downloads;
mod engine;
mod library;
pub mod protocol;
pub(crate) mod store;
pub use client::{CloudClient, SyncError};
pub use engine::CloudSyncContext;
use std::time::Duration;
pub use store::apply_to_connection;
use tokio::{
    runtime::Handle,
    sync::{mpsc, watch},
    task::JoinHandle,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CloudSyncStatus {
    #[default]
    Idle,
    Running,
    Failed(String),
    Stopped,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SyncPhase {
    #[default]
    Checking,
    Discovering,
    Uploading,
    Processing,
    Updating,
    Waiting,
    Complete,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CloudSyncProgress {
    pub phase: SyncPhase,
    pub file_name: Option<String>,
    pub bytes_sent: u64,
    pub bytes_total: u64,
    pub queued: u64,
    pub completed: u64,
    pub failed: u64,
    pub retry_at: Option<u64>,
}
impl CloudSyncProgress {
    pub fn percent(&self) -> f32 {
        if self.bytes_total == 0 {
            return 0.;
        }
        (self.bytes_sent.min(self.bytes_total) as f64 / self.bytes_total as f64 * 100.) as f32
    }
}

pub struct CloudWorker {
    wake: mpsc::Sender<u8>,
    shutdown: watch::Sender<bool>,
    status: watch::Sender<CloudSyncStatus>,
    progress: watch::Sender<CloudSyncProgress>,
    task: Option<JoinHandle<()>>,
}

impl CloudWorker {
    pub fn start(context: CloudSyncContext, auto_sync: bool, runtime: &Handle) -> Self {
        let (wake, mut requests) = mpsc::channel(16);
        let (shutdown, mut stopping) = watch::channel(false);
        let (status, _) = watch::channel(CloudSyncStatus::Idle);
        let worker_status = status.clone();
        let (progress, _) = watch::channel(CloudSyncProgress::default());
        let worker_progress = progress.clone();
        let mut library = context.files.subscribe();
        let task = runtime.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut requested = 1u8;
            let mut force_retry=false;
            let mut cooldown:Option<tokio::time::Instant>=None;
            let mut last_cycle:Option<tokio::time::Instant>=None;
            loop {
                let manual = tokio::select! {
                    biased;
                    _ = stopping.changed() => break,
                    change = library.recv() => match change {
                        Ok(change) if change.artwork_only => 0,
                        _ => 1,
                    },
                    request = requests.recv() => match request { Some(manual) => manual, None => break },
                    _ = interval.tick() => 0,
                };
                requested = requested.max(manual);
                force_retry |= manual==2;
                while let Ok(mode) = requests.try_recv() { requested = requested.max(mode); force_retry |= mode==2; }
                let now=tokio::time::Instant::now();
                if cooldown.is_some_and(|until|until>now) {continue;}
                if manual<2 && last_cycle.is_some_and(|last|now.duration_since(last)<Duration::from_secs(5)) {continue;}
                cooldown=None;last_cycle=Some(now);
                worker_progress.send_modify(|p|p.retry_at=None);
                if force_retry {
                    force_retry=false;
                    let _=sqlx::query("UPDATE cloud_outbox SET retry_at=0,attempts=0 WHERE connection_id=? AND state='pending'").bind(&context.connection_id).execute(&context.db.pool).await;
                }
                worker_status.send_replace(CloudSyncStatus::Running);
                let result = tokio::select! {
                    biased;
                    _ = stopping.changed() => break,
                    result = engine::cycle(&context, auto_sync || requested == 2, auto_sync || requested > 0, &worker_progress) => result,
                };
                if requested > 0 {
                    if let Ok(0) = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM cloud_outbox WHERE connection_id=? AND state='pending' AND (? OR json_extract(payload,'$.job')!='upload')").bind(&context.connection_id).bind(requested == 2).fetch_one(&context.db.pool).await { requested = 0; }
                }
                worker_status.send_replace(match result {
                    Ok(()) => CloudSyncStatus::Idle,
                    Err(error) if error.status==Some(429) || (error.status==Some(503) && error.retry_after>0) => {
                        let wait=error.retry_after.clamp(3,86460);
                        cooldown=Some(tokio::time::Instant::now()+Duration::from_secs(wait));
                        let deadline=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()+wait;
                        worker_progress.send_modify(|p|{p.phase=SyncPhase::Waiting;p.file_name=None;p.bytes_sent=0;p.bytes_total=0;p.retry_at=Some(deadline);});
                        if error.status==Some(503) { CloudSyncStatus::Failed(error.to_string()) } else { CloudSyncStatus::Idle }
                    },
                    Err(error) => {worker_progress.send_modify(|p| {p.phase=SyncPhase::Waiting;p.bytes_sent=0;p.bytes_total=0;p.file_name=None;});CloudSyncStatus::Failed(error.to_string())},
                });
            }
            worker_status.send_replace(CloudSyncStatus::Stopped);
        });
        Self {
            wake,
            shutdown,
            status,
            progress,
            task: Some(task),
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<CloudSyncStatus> {
        self.status.subscribe()
    }

    pub fn subscribe_progress(&self) -> watch::Receiver<CloudSyncProgress> {
        self.progress.subscribe()
    }

    pub fn wake(&self) -> bool {
        match self.wake.try_send(2) {
            Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => true,
            Err(mpsc::error::TrySendError::Closed(_)) => false,
        }
    }

    pub fn wake_edits(&self) {
        let _ = self.wake.try_send(1);
    }

    pub async fn stop(mut self) {
        self.shutdown.send_replace(true);
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
        self.status.send_replace(CloudSyncStatus::Stopped);
    }
}

impl Drop for CloudWorker {
    fn drop(&mut self) {
        self.shutdown.send_replace(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.status.send_replace(CloudSyncStatus::Stopped);
    }
}
