use super::media::{MediaTarget, MediaTrack, PlaylistOccurrence, SelectedTrack};
use crate::{
    services::AppServices,
    ui::{hooks::use_playback, theme::use_audexis_colors},
};
use audexis_core::database::songs::SongRow;
use freya::{prelude::*, router::RouterContext};
use std::{collections::HashSet, sync::Arc};

mod model;
mod row;
mod scope;
use model::{ClickTracker, ModifierTracker, PointerPress, Selection};
pub(crate) use row::SelectableTrack;
pub(crate) use scope::TrackSelectionScope;

#[derive(Clone, Copy)]
struct SelectionContext {
    selection: State<Selection>,
    items: State<Arc<Vec<SelectedTrack>>>,
    modifiers: State<ModifierTracker>,
    select_all: State<Option<EventHandler<bool>>>,
    clicks: State<ClickTracker>,
}

pub(crate) fn song_items(
    tracks: &Arc<Vec<SongRow>>,
    playlist: Option<i64>,
) -> Arc<Vec<SelectedTrack>> {
    let cache = use_hook(|| {
        std::rc::Rc::new(std::cell::RefCell::new(
            None::<(Arc<Vec<SongRow>>, Option<i64>, Arc<Vec<SelectedTrack>>)>,
        ))
    });
    let mut cache = cache.borrow_mut();
    if let Some((previous, previous_playlist, items)) = cache.as_ref() {
        if Arc::ptr_eq(previous, tracks) && *previous_playlist == playlist {
            return items.clone();
        }
    }
    let items: Arc<Vec<SelectedTrack>> = Arc::new(
        tracks
            .iter()
            .map(|song| SelectedTrack {
                track: MediaTrack {
                    id: song.id,
                    path: song.path.clone(),
                },
                queue_id: None,
                playlist: playlist.zip(song.playlist_ord).map(|(playlist, ord)| {
                    PlaylistOccurrence {
                        playlist,
                        ord,
                        file: song.id,
                    }
                }),
            })
            .collect(),
    );
    *cache = Some((tracks.clone(), playlist, items.clone()));
    items
}
