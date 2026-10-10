use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    Off,
    Queue,
    Track,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaybackModes {
    pub repeat_mode: RepeatMode,
    pub shuffled: bool,
}
