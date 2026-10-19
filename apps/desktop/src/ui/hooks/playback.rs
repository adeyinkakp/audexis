use crate::services::AppServices;
use audexis_audio::{PlaybackState, PlayerCmd, PlayerHandle, QueueInfo};
use audexis_core::database::songs::SongQuery;
use freya::prelude::*;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq)]
pub enum PlaybackPanelView {
    Queue,
    Lyrics,
}

#[derive(Clone)]
pub struct Playback {
    pub state: State<PlaybackState>,
    pub queue: State<QueueInfo>,
    pub panel: State<Option<PlaybackPanelView>>,
    pub error: State<Option<String>>,
    player: Option<PlayerHandle>,
    library_request: State<Option<(u64, SongQuery, i64)>>,
}
impl Playback {
    pub fn play_library(&self, query: SongQuery, selected_id: i64) {
        let mut request = self.library_request;
        let generation = request
            .peek()
            .as_ref()
            .map_or(0, |(n, _, _)| n.wrapping_add(1));
        request.set(Some((generation, query, selected_id)));
    }

    pub fn toggle_panel(&self, view: PlaybackPanelView) {
        let mut panel = self.panel;
        let next = if *panel.peek() == Some(view) {
            None
        } else {
            Some(view)
        };
        panel.set(next);
    }
    pub fn available(&self) -> bool {
        self.player.is_some()
    }
    pub fn snapshot(&self) -> PlaybackState {
        self.state.read().clone()
    }
    pub fn send(&self, command: PlayerCmd) {
        let mut request = self.library_request;
        request.set_if_modified(None);
        let result = self
            .player
            .as_ref()
            .ok_or_else(|| "Audio is unavailable".to_string())
            .and_then(|player| player.send(command));
        let mut error = self.error;
        error.set(result.err());
    }
}
pub fn use_playback() -> Playback {
    use_consume::<Playback>()
}

pub(super) fn provide_playback(services: Arc<AppServices>) {
    let initial = services
        .player
        .as_ref()
        .map(|p| p.subscribe().borrow().clone())
        .unwrap_or_else(|| PlaybackState {
            error: services.audio_error.clone(),
            ..Default::default()
        });
    let mut state = use_state(move || initial);
    let error = use_state(|| None);
    let panel = use_state(|| None);
    let mut queue = use_state(|| {
        services
            .player
            .as_ref()
            .map(|p| p.subscribe_queue().borrow().clone())
            .unwrap_or_default()
    });
    let queue_player = services.player.clone();
    use_future(move || {
        let player = queue_player.clone();
        async move {
            if let Some(player) = player {
                let mut receiver = player.subscribe_queue();
                queue.set(receiver.borrow_and_update().clone());
                while receiver.changed().await.is_ok() {
                    queue.set(receiver.borrow_and_update().clone());
                }
            }
        }
    });
    let library_request = use_state(|| None::<(u64, SongQuery, i64)>);
    let db = services.db.clone();
    let query_player = services.player.clone();
    let runtime = use_consume::<tokio::runtime::Handle>();
    let mut query_error = error;
    let mut library_loader = use_hook(move || {
        FutureTask::create(move || {
            let request = library_request.peek().clone();
            let db = db.clone();
            let player = query_player.clone();
            let runtime = runtime.clone();
            async move {
                let Some((_, query, selected)) = request.clone() else {
                    return;
                };
                let task = runtime.spawn(async move { db.songs_queue(&query, selected).await });
                let _abort = AbortLibraryLoad(task.abort_handle());
                let result = task.await.map_err(|e| e.to_string()).and_then(|r| r);
                if *library_request.peek() != request {
                    return;
                }
                let result = result.and_then(|selection| {
                    let player = player
                        .as_ref()
                        .ok_or_else(|| "Audio is unavailable".to_string())?;
                    player.send(PlayerCmd::LoadAt {
                        tracks: selection
                            .tracks
                            .into_iter()
                            .map(|track| audexis_audio::queue_track::UnresolvedTrack {
                                id: track.id,
                                path: track.path,
                                occurrence: None,
                            })
                            .collect(),
                        index: selection.index,
                    })
                });
                query_error.set(result.err());
            }
        })
    });
    use_side_effect(move || {
        let request = library_request.read().clone();
        library_loader.cancel();
        if request.is_some() {
            library_loader.start();
        }
    });
    let player = services.player.clone();
    use_provide_context(move || Playback {
        state,
        queue,
        panel,
        error,
        player,
        library_request,
    });
    use_future(move || {
        let player = services.player.clone();
        async move {
            if let Some(player) = player {
                let mut receiver = player.subscribe();
                let initial = receiver.borrow_and_update().clone();
                state.set(initial);
                while receiver.changed().await.is_ok() {
                    let next = receiver.borrow_and_update().clone();
                    state.set(next);
                }
            }
        }
    });
}

struct AbortLibraryLoad(tokio::task::AbortHandle);
impl Drop for AbortLibraryLoad {
    fn drop(&mut self) {
        self.0.abort();
    }
}
