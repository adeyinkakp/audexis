use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback};

pub(crate) struct MediaSession(pub Option<MediaControls>);

impl MediaSession {
    pub fn attach(
        &mut self,
        handler: impl Fn(MediaControlEvent) + Send + 'static,
    ) -> Result<(), String> {
        if let Some(controls) = &mut self.0 {
            controls.attach(handler).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn set_metadata(&mut self, metadata: MediaMetadata<'_>) -> Result<(), String> {
        if let Some(controls) = &mut self.0 {
            controls.set_metadata(metadata).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn set_playback(&mut self, playback: MediaPlayback) -> Result<(), String> {
        if let Some(controls) = &mut self.0 {
            controls.set_playback(playback).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn detach(&mut self) {
        if let Some(controls) = &mut self.0 {
            let _ = controls.detach();
        }
    }
}
