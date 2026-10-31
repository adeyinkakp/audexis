mod decoder_guard;
pub(crate) mod duration;
pub mod modes;
mod playback;
mod progress;
mod queue;
pub mod queue_track;
pub mod streaming;
mod types;
mod worker;

pub use modes::{PlaybackModes, RepeatMode};
pub use playback::AudioPlayer;
pub use types::{AudioPlayerError, PartialMetadata, PlayerCmd};

mod play_count;

mod listening_time;

pub mod lyrics_preview;

pub mod equalizer;

mod events;
mod media;
pub use events::{AudioEvent, PlaybackState, QueueInfo};
pub use playback::PlayerHandle;
