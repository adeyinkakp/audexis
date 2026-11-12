use super::pages::cloud::CloudLibrary;
use super::pages::collections::{
    AlbumForTrack, Albums, Artists, Favorites, Playlist, Playlists, Search,
};
use super::pages::missing::Missing;
use super::pages::rewind::Rewind;
use super::pages::{home::Home, songs::Songs};
use super::shell::Shell;
use freya::{prelude::*, router::*};
#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Shell)]
        #[route("/")]
        Home,
        #[route("/cloud")]
        CloudLibrary,
        #[route("/songs")]
        Songs,
        #[route("/artists")]
        Artists,
        #[route("/albums")]
        Albums,
        #[route("/albums/track/:file_id")]
        AlbumForTrack {file_id:i64},
        #[route("/playlists")]
        Playlists,
        #[route("/playlists/:playlist_id")]
        Playlist { playlist_id: i64 },
        #[route("/favorites")]
        Favorites,
        #[route("/search")]
        Search,
        #[route("/rewind")]
        Rewind,
        #[route("/missing")]
        Missing,
}
