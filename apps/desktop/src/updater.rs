use cargo_packager_updater::Update;
mod manifest;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::sync::watch;

#[derive(Clone, Default)]
pub enum Phase {
    #[default]
    Idle,
    Checking,
    Ready,
    Downloading,
    Installed,
    Error(String),
}

#[derive(Clone, Default)]
pub struct Status {
    pub update: Option<Arc<Update>>,
    pub phase: Phase,
}

pub struct Updates {
    pub status: watch::Sender<Status>,
    active: AtomicBool,
}

pub fn updates() -> Arc<Updates> {
    static UPDATES: OnceLock<Arc<Updates>> = OnceLock::new();
    UPDATES
        .get_or_init(|| {
            Arc::new(Updates {
                status: watch::channel(Status::default()).0,
                active: AtomicBool::new(false),
            })
        })
        .clone()
}

pub fn start(services: Arc<crate::services::AppServices>, runtime: &tokio::runtime::Handle) {
    let weak = Arc::downgrade(&services);
    let mut changes = services.config_updates.subscribe();
    runtime.spawn(async move {
        let mut timer = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut was_enabled = false;
        loop {
            let scheduled = tokio::select! {
                _ = timer.tick() => true,
                event = changes.recv() => {
                    if matches!(event, Err(tokio::sync::broadcast::error::RecvError::Closed)) { break; }
                    false
                }
            };
            let Some(services) = weak.upgrade() else { break; };
            let enabled = services.user_config.lock().map(|config| config.auto_update_checks).unwrap_or(false);
            if enabled && (scheduled || !was_enabled) { updates().check().await; }
            was_enabled = enabled;
        }
    });
}

impl Updates {
    pub async fn check(self: Arc<Self>) {
        if matches!(self.status.borrow().phase, Phase::Installed) {
            return;
        }
        if self.active.swap(true, Ordering::AcqRel) {
            return;
        }
        self.status.send_replace(Status {
            phase: Phase::Checking,
            ..Default::default()
        });
        let result = manifest::check().await;
        let status = match result {
            Ok(Some(mut update)) => {
                update.timeout = Some(Duration::from_secs(10 * 60));
                Status {
                    update: Some(Arc::new(update)),
                    phase: Phase::Ready,
                }
            }
            Ok(None) => Status::default(),
            Err(error) => Status {
                phase: Phase::Error(error.to_string()),
                ..Default::default()
            },
        };
        self.status.send_replace(status);
        self.active.store(false, Ordering::Release);
    }

    pub async fn install(self: Arc<Self>, update: Arc<Update>) {
        if self.active.swap(true, Ordering::AcqRel) {
            return;
        }
        self.status.send_replace(Status {
            update: Some(update.clone()),
            phase: Phase::Downloading,
        });
        let task_update = update.clone();
        let result = tokio::task::spawn_blocking(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                if !can_install() {
                    return Err(std::io::Error::other(
                        "Launch the packaged Audexis.app or AppImage to install updates.",
                    )
                    .into());
                }
                task_update.download_and_install()?;
                restart()?;
                Ok(())
            },
        )
        .await;
        let phase = match result {
            Ok(Ok(())) => Phase::Installed,
            Ok(Err(error)) => Phase::Error(error.to_string()),
            Err(error) => Phase::Error(error.to_string()),
        };
        self.status.send_replace(Status {
            update: Some(update),
            phase,
        });
        self.active.store(false, Ordering::Release);
    }
}

fn can_install() -> bool {
    if cfg!(target_os = "linux") {
        return std::env::var_os("APPIMAGE").is_some();
    }
    if cfg!(target_os = "macos") {
        return std::env::current_exe().ok().is_some_and(|path| {
            path.components()
                .any(|part| part.as_os_str().to_string_lossy().ends_with(".app"))
        });
    }
    cfg!(target_os = "windows")
}

fn restart() -> std::io::Result<()> {
    let executable = if cfg!(target_os = "linux") {
        std::env::var_os("APPIMAGE")
            .map(std::path::PathBuf::from)
            .map(Ok)
            .unwrap_or_else(std::env::current_exe)?
    } else {
        std::env::current_exe()?
    };
    std::process::Command::new(executable)
        .env_remove("APPIMAGE")
        .env_remove("APPDIR")
        .spawn()?;
    Ok(())
}
