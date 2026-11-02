use audexis_protocol::Track;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub(super) enum RemoteAction {}

#[derive(Default)]
pub(super) struct MobileMediaControls;

impl MobileMediaControls {
    pub(super) fn select(&mut self, _track: Track) -> Result<(), String> {
        Ok(())
    }
    pub(super) fn artwork(&mut self, _path: PathBuf) {}
    pub(super) fn queue(&mut self, _index: Option<usize>, _count: usize) {}
    pub(super) fn update(
        &mut self,
        _playing: bool,
        _position_ms: u64,
        _duration_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
    pub(super) fn drain(&self) -> Vec<RemoteAction> {
        Vec::new()
    }
    pub(super) fn clear(&mut self) {}
}
