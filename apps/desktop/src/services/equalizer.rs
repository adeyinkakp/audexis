use super::*;
use audexis_audio::equalizer::Settings;

impl AppServices {
    pub fn set_equalizer(&self, settings: Settings) -> Result<(), String> {
        settings.validate()?;
        if let Some(audio) = &self.audio {
            let audio = audio.lock().map_err(|_| "Audio unavailable")?;
            *audio
                .equalizer
                .lock()
                .map_err(|_| "Equalizer unavailable")? = settings;
        }
        self.equalizer_updates.send_replace(settings);
        Ok(())
    }

    pub fn start_equalizer_persistence(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        let mut updates = self.equalizer_updates.subscribe();
        tokio::spawn(async move {
            while updates.changed().await.is_ok() {
                loop {
                    tokio::select! {
                        changed = updates.changed() => {
                            if changed.is_err() { return; }
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_millis(250)) => break,
                    }
                }
                let settings = *updates.borrow_and_update();
                let Some(services) = weak.upgrade() else {
                    return;
                };
                let status = services.equalizer_error.clone();
                let result = tokio::task::spawn_blocking(move || {
                    services.update_config(|config| config.equalizer = settings)
                })
                .await;
                status.send_replace(match result {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(format!("Could not save equalizer: {error}")),
                    Err(_) => Some("Could not save equalizer settings.".into()),
                });
            }
        });
    }
}
