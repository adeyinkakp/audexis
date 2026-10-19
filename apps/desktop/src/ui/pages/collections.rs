use crate::{
    services::AppServices,
    ui::{
        components::{Artwork, SmoothScroll},
        hooks::use_library,
        theme::use_audexis_colors,
    },
};
use audexis_core::database::collections::{
    Collection, CollectionKind, TrackFilter, browse_library_with_pool,
};
use freya::{icons::lucide, prelude::*};
use std::sync::Arc;
mod page;
mod search;
use crate::ui::components::TrackList;
use page::LibraryPage;
use search::SearchCollections;
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Artists,
    Albums,
    Playlists,
    Favorites,
    Search,
}
impl Kind {
    fn title(self) -> &'static str {
        match self {
            Self::Artists => "Artists",
            Self::Albums => "Albums",
            Self::Playlists => "Playlists",
            Self::Favorites => "Favorites",
            Self::Search => "Search",
        }
    }
    fn collection(self) -> CollectionKind {
        match self {
            Self::Artists => CollectionKind::Artists,
            Self::Playlists => CollectionKind::Playlists,
            _ => CollectionKind::Albums,
        }
    }
}
macro_rules! page {
    ($name:ident, $kind:ident) => {
        #[derive(PartialEq)]
        pub struct $name;
        impl Component for $name {
            fn render(&self) -> impl IntoElement {
                LibraryPage {
                    kind: Kind::$kind,
                    initial: None,
                }
            }
        }
    };
}
page!(Artists, Artists);
page!(Albums, Albums);
page!(Playlists, Playlists);
page!(Favorites, Favorites);
page!(Search, Search);
#[derive(PartialEq)]
pub struct AlbumForTrack {
    pub file_id: i64,
}
impl Component for AlbumForTrack {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let id = self.file_id;
        let mut loader = use_hook(move || {
            FutureTask::create(move || {
                let db = services.db.clone();
                let runtime = runtime.clone();
                async move {
                    runtime
                        .spawn(async move {
                            let mut album = db
                                .album_for_track(id)
                                .await
                                .map_err(|e| e.to_string())?
                                .ok_or("This song is no longer in your library")?;
                            let tracks = db
                                .collection_tracks(TrackFilter {
                                    album: Some(album.name.clone()),
                                    artist: (!album.name.is_empty()).then(|| album.artist.clone()),
                                    ..Default::default()
                                })
                                .await
                                .map_err(|e| e.to_string())?;
                            album.track_count = tracks.len() as i64;
                            Ok::<_, String>(album)
                        })
                        .await
                        .map_err(|e| e.to_string())?
                }
            })
        });
        use_side_effect_with_deps(&id, move |_| loader.start());
        let state = loader.state();
        match state.try_as_fulfilled() {
            Some(Ok(album)) => LibraryPage {
                kind: Kind::Albums,
                initial: Some(album.clone()),
            }
            .into_element(),
            Some(Err(e)) => label().text(e.clone()).margin(28.).into_element(),
            None => label().text("Loading album…").margin(28.).into_element(),
        }
    }
    fn render_key(&self) -> DiffKey {
        (&self.file_id).into()
    }
}
#[derive(PartialEq)]
pub struct Playlist {
    pub playlist_id: i64,
}
impl Component for Playlist {
    fn render(&self) -> impl IntoElement {
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let id = self.playlist_id;
        let mut loader = use_hook(move || {
            FutureTask::create(move || {
                let db = services.db.clone();
                let runtime = runtime.clone();
                async move {
                    runtime
                        .spawn(async move {
                            sqlx::query_as::<_, (String, Option<i64>, i64)>(concat!(
                                "SELECT p.name,MIN(f.id),COUNT(f.id) FROM playlists p LEFT JOIN ",
                                "playlist_tracks pt ON pt.playlist_id=p.id LEFT JOIN files f ON ",
                                "f.id=pt.file_id AND f.missing_since IS NULL WHERE p.id=? GROUP BY p.id",
                            ))
                            .bind(id)
                            .fetch_optional(&db.pool)
                            .await
                            .map_err(|e| e.to_string())
                        })
                        .await
                        .map_err(|e| e.to_string())?
                }
            })
        });
        use_side_effect_with_deps(&id, move |_| {
            loader.cancel();
            loader.start();
        });
        let state = loader.state();
        match state.try_as_fulfilled() {
            Some(Ok(Some((name, file_id, count)))) => LibraryPage {
                kind: Kind::Playlists,
                initial: Some(Collection {
                    name: name.clone(),
                    artist: String::new(),
                    file_id: *file_id,
                    playlist_id: Some(id),
                    track_count: *count,
                }),
            }
            .into_element(),
            Some(Ok(None)) => label()
                .text("Playlist not found.")
                .margin(28.)
                .into_element(),
            Some(Err(e)) => label().text(e.clone()).margin(28.).into_element(),
            None => label().text("Loading playlist…").margin(28.).into_element(),
        }
    }
    fn render_key(&self) -> DiffKey {
        (&self.playlist_id).into()
    }
}
fn display_name(name: &str) -> String {
    if name.is_empty() {
        "Uncategorized".into()
    } else {
        name.into()
    }
}
fn heading(text: &str, size: f32) -> impl IntoElement {
    label()
        .text(crate::ui::display_text(text.to_owned()))
        .font_size(size)
        .font_weight(FontWeight::BOLD)
}
