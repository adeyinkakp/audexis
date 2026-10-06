pub(crate) mod duration;
pub mod modes;
mod playback;
mod progress;
mod queue;
pub mod queue_track;
mod types;
mod worker;

pub use modes::{PlaybackModes, RepeatMode};
pub use playback::AudioPlayer;
pub use types::{AudioPlayerError, PartialMetadata, PlayerCmd};

mod play_count;

mod listening_time;

pub(crate) mod lyrics_preview;

pub mod equalizer;
