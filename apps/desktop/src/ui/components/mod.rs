mod now_playing;
mod sidebar;

pub use now_playing::NowPlaying;
pub use sidebar::Sidebar;

mod discovery_shelf;

pub use discovery_shelf::DiscoveryShelf;

mod artwork;
pub use artwork::Artwork;

mod smooth_scroll;
pub use smooth_scroll::SmoothScroll;

mod visibility;
pub use visibility::VisibilityViewport;

mod library_controls;
pub use library_controls::LibraryControls;

mod compact_seek;

mod playback_panel;

pub use playback_panel::PlaybackPanel;
mod lyrics_scroll;

mod lyrics_line;

mod lyrics_gap;

pub mod media;

mod queue_drag;

pub mod song_table;

mod sortable_motion;

mod track_list;
pub(crate) use track_list::TrackList;

mod artwork_card;
pub(crate) use artwork_card::ArtworkCard;
mod playlist_form;
pub(crate) use playlist_form::PlaylistForm;

mod infinite_scroll;
pub(crate) use infinite_scroll::InfiniteScroll;

pub(crate) mod track_selection;
