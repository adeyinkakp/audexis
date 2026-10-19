use crate::services::AppServices;
use audexis_audio::AudioEvent;
use audexis_core::database::{
    Database,
    discovery::{DiscoveryTrack, HomeDiscovery},
    types::DatabaseMediaFile,
};
use freya::prelude::*;
use std::{collections::HashMap, sync::Arc};
use tokio::{runtime::Handle, sync::mpsc};

#[derive(Clone)]
pub struct Query<T> {
    pub data: Option<Arc<T>>,
    pub loading: bool,
    pub error: Option<String>,
}
impl<T> Default for Query<T> {
    fn default() -> Self {
        Self {
            data: None,
            loading: true,
            error: None,
        }
    }
}
impl<T> Query<T> {
    fn complete(&mut self, result: Result<T, String>) {
        self.loading = false;
        match result {
            Ok(data) => {
                self.data = Some(Arc::new(data));
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
}
#[derive(Clone)]
enum Request {
    Discovery,
    Files(Vec<i64>),
    Songs(Vec<i64>),
    Refresh,
}
#[derive(Clone)]
pub struct Library {
    discovery: State<Query<HomeDiscovery>>,
    watcher: State<audexis_core::file_watcher::WatcherStatus>,
    files: State<HashMap<Vec<i64>, Query<Vec<DatabaseMediaFile>>>>,
    songs: State<HashMap<Vec<i64>, Query<Vec<DiscoveryTrack>>>>,
    refresh_pending: State<bool>,
    revision: State<u64>,
    requests: mpsc::UnboundedSender<Request>,
}
impl Library {
    pub fn watcher_status(&self) -> audexis_core::file_watcher::WatcherStatus {
        self.watcher.read().clone()
    }
    pub fn refresh(&self) {
        let mut pending = self.refresh_pending;
        if !*pending.peek() {
            pending.set(true);
            let _ = self.requests.send(Request::Refresh);
        }
    }
    pub fn revision(&self) -> u64 {
        *self.revision.read()
    }
    pub fn discovery(&self) -> Query<HomeDiscovery> {
        self.discovery.read().clone()
    }
    fn ensure_files(&mut self, ids: Vec<i64>) {
        if !self.files.peek().contains_key(&ids) {
            self.files.write().insert(ids.clone(), Query::default());
            let _ = self.requests.send(Request::Files(ids));
        }
    }
    fn ensure_songs(&mut self, ids: Vec<i64>) {
        if !self.songs.peek().contains_key(&ids) {
            self.songs.write().insert(ids.clone(), Query::default());
            let _ = self.requests.send(Request::Songs(ids));
        }
    }
}
pub fn use_library() -> Library {
    use_consume::<Library>()
}
pub fn use_home_discovery() -> Query<HomeDiscovery> {
    use_library().discovery()
}
fn canonical_ids(ids: &[i64]) -> Vec<i64> {
    let mut ids: Vec<_> = ids.iter().copied().filter(|id| *id > 0).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}
pub fn use_files(ids: &[i64]) -> Query<Vec<DatabaseMediaFile>> {
    let mut library = use_library();
    let ids = canonical_ids(ids);
    let resource = library.files;
    use_side_effect_with_deps(&ids, move |ids| library.ensure_files(ids.clone()));
    resource.read().get(&ids).cloned().unwrap_or_default()
}
pub fn use_songs(ids: &[i64]) -> Query<Vec<DiscoveryTrack>> {
    let mut library = use_library();
    let ids = canonical_ids(ids);
    let resource = library.songs;
    use_side_effect_with_deps(&ids, move |ids| library.ensure_songs(ids.clone()));
    resource.read().get(&ids).cloned().unwrap_or_default()
}

async fn load(store: &mut Library, db: &Database, runtime: &Handle, request: Request) {
    match request {
        Request::Discovery => {
            store.discovery.write().loading = true;
            let db = db.clone();
            let result = runtime
                .spawn(async move { db.home_discovery().await.map_err(|e| e.to_string()) })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);
            store.discovery.write().complete(result);
        }
        Request::Files(ids) => {
            store.files.write().entry(ids.clone()).or_default().loading = true;
            let db = db.clone();
            let requested = ids.clone();
            let result = runtime
                .spawn(async move { db.files_by_ids(&requested).await.map_err(|e| e.to_string()) })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);
            store.files.write().entry(ids).or_default().complete(result);
        }
        Request::Songs(ids) => {
            store.songs.write().entry(ids.clone()).or_default().loading = true;
            let db = db.clone();
            let requested = ids.clone();
            let result = runtime
                .spawn(async move { db.songs_by_ids(&requested).await.map_err(|e| e.to_string()) })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);
            store.songs.write().entry(ids).or_default().complete(result);
        }
        Request::Refresh => unreachable!("expanded by the root loader"),
    }
}

pub(super) fn provide_library(services: Arc<AppServices>, runtime: Handle) {
    let watcher = use_state(|| {
        services
            .watcher
            .as_ref()
            .map(|watcher| watcher.subscribe().borrow().clone())
            .unwrap_or_else(|| audexis_core::file_watcher::WatcherStatus {
                error: services.watcher_error.clone(),
                ..Default::default()
            })
    });
    let watcher_service = services.watcher.clone();
    use_future(move || {
        let service = watcher_service.clone();
        let mut status = watcher;
        async move {
            if let Some(service) = service {
                let mut receiver = service.subscribe();
                status.set(receiver.borrow_and_update().clone());
                while receiver.changed().await.is_ok() {
                    status.set(receiver.borrow_and_update().clone());
                }
            }
        }
    });
    let discovery = use_state(Query::default);
    let files = use_state(HashMap::new);
    let songs = use_state(HashMap::new);
    let refresh_pending = use_state(|| false);
    let revision = use_state(|| 0);
    let (sender, mut receiver) = use_hook(|| {
        let (sender, receiver) = mpsc::unbounded_channel();
        (sender, State::create(Some(receiver)))
    });
    let store = use_provide_context(move || Library {
        watcher,
        discovery,
        files,
        songs,
        refresh_pending,
        revision,
        requests: sender,
    });
    let loader_store = store.clone();
    let db = services.db.clone();
    let artwork = services.artwork.clone();
    use_future(move || {
        let receiver = receiver.take();
        let mut store = loader_store.clone();
        let db = db.clone();
        let runtime = runtime.clone();
        let artwork = artwork.clone();
        async move {
            let Some(mut receiver) = receiver else {
                return;
            };
            load(&mut store, &db, &runtime, Request::Discovery).await;
            while let Some(request) = receiver.recv().await {
                if matches!(request, Request::Refresh) {
                    store.refresh_pending.set(false);
                    artwork.invalidate().await;
                    let revision = *store.revision.peek();
                    store.revision.set(revision.wrapping_add(1));
                    let files: Vec<_> = store.files.peek().keys().cloned().collect();
                    let songs: Vec<_> = store.songs.peek().keys().cloned().collect();
                    load(&mut store, &db, &runtime, Request::Discovery).await;
                    for ids in files {
                        load(&mut store, &db, &runtime, Request::Files(ids)).await;
                    }
                    for ids in songs {
                        load(&mut store, &db, &runtime, Request::Songs(ids)).await;
                    }
                } else {
                    load(&mut store, &db, &runtime, request).await;
                }
            }
        }
    });
    let changes_store = store.clone();
    let mut changes = use_hook(|| State::create(Some(services.library_files.subscribe())));
    use_future(move || {
        let receiver = changes.take();
        let store = changes_store.clone();
        async move {
            let Some(mut receiver) = receiver else {
                return;
            };
            loop {
                match receiver.recv().await {
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        store.refresh()
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    });
    use_future(move || {
        let player = services.player.clone();
        let store = store.clone();
        async move {
            if let Some(player) = player {
                let mut receiver = player.subscribe_events();
                loop {
                    match receiver.recv().await {
                        Ok(
                            AudioEvent::PlayCountChanged(_)
                            | AudioEvent::ListeningHistoryChanged(_),
                        ) => {
                            let _ = store.requests.send(Request::Discovery);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            let _ = store.requests.send(Request::Discovery);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        _ => {}
                    }
                }
            }
        }
    });
}
