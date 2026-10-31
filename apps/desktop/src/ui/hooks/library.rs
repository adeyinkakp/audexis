mod provider;
pub(super) use provider::provide_library;

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
impl<T: PartialEq> Query<T> {
    fn needs_update(&self, result: &Result<T, String>) -> bool {
        self.loading
            || match result {
                Ok(data) => self.data.as_deref() != Some(data) || self.error.is_some(),
                Err(error) => self.error.as_ref() != Some(error),
            }
    }
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
    artwork_revision: State<u64>,
    connection: State<Option<String>>,
    refresh_generation: State<u64>,
    requests: mpsc::UnboundedSender<Request>,
}
impl Library {
    pub fn watcher_status(&self) -> audexis_core::file_watcher::WatcherStatus {
        self.watcher.read().clone()
    }
    pub fn refresh(&self) {
        let mut generation = self.refresh_generation;
        let next = generation.peek().wrapping_add(1);
        generation.set(next);
        let mut pending = self.refresh_pending;
        if !*pending.peek() {
            pending.set(true);
            let _ = self.requests.send(Request::Refresh);
        }
    }
    pub fn revision(&self) -> u64 {
        *self.revision.read()
    }
    pub fn connection(&self) -> Option<String> {
        self.connection.read().clone()
    }
    pub fn artwork_revision(&self) -> u64 {
        *self.artwork_revision.read()
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
            let db = db.clone();
            let result = runtime
                .spawn(async move { db.home_discovery().await.map_err(|e| e.to_string()) })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);
            if store.discovery.peek().needs_update(&result) {
                store.discovery.write().complete(result);
            }
        }
        Request::Files(ids) => {
            let db = db.clone();
            let requested = ids.clone();
            let result = runtime
                .spawn(async move { db.files_by_ids(&requested).await.map_err(|e| e.to_string()) })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);
            if store
                .files
                .peek()
                .get(&ids)
                .is_none_or(|query| query.needs_update(&result))
            {
                store.files.write().entry(ids).or_default().complete(result);
            }
        }
        Request::Songs(ids) => {
            let db = db.clone();
            let requested = ids.clone();
            let result = runtime
                .spawn(async move { db.songs_by_ids(&requested).await.map_err(|e| e.to_string()) })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);
            if store
                .songs
                .peek()
                .get(&ids)
                .is_none_or(|query| query.needs_update(&result))
            {
                store.songs.write().entry(ids).or_default().complete(result);
            }
        }
        Request::Refresh => unreachable!("expanded by the root loader"),
    }
}
