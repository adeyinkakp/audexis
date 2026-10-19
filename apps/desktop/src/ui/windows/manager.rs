use freya::winit::window::WindowId;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};

pub struct WindowManager {
    settings_open: AtomicBool,
    settings_window: Mutex<Option<WindowId>>,
}

impl Default for WindowManager {
    fn default() -> Self {
        Self {
            settings_open: AtomicBool::new(false),
            settings_window: Mutex::new(None),
        }
    }
}

impl WindowManager {
    pub fn open_settings(&self) -> Option<WindowId> {
        if let Ok(window) = self.settings_window.lock() {
            if let Some(window_id) = *window {
                return Some(window_id);
            }
        }
        let opened = self
            .settings_open
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();
        if opened {
            None
        } else {
            self.settings_window.lock().ok().and_then(|window| *window)
        }
    }

    pub fn set_settings_window(&self, window_id: WindowId) {
        if let Ok(mut window) = self.settings_window.lock() {
            *window = Some(window_id);
        }
    }

    pub fn close_settings(&self, window_id: WindowId) {
        if let Ok(mut window) = self.settings_window.lock() {
            if *window == Some(window_id) {
                *window = None;
                self.settings_open.store(false, Ordering::Release);
            }
        }
    }

    #[cfg(test)]
    pub fn reset_settings(&self) {
        if let Ok(mut window) = self.settings_window.lock() {
            *window = None;
        }
        self.settings_open.store(false, Ordering::Release);
    }
}

static WINDOW_MANAGER: OnceLock<Arc<WindowManager>> = OnceLock::new();

pub fn window_manager() -> Arc<WindowManager> {
    WINDOW_MANAGER
        .get_or_init(|| Arc::new(WindowManager::default()))
        .clone()
}
