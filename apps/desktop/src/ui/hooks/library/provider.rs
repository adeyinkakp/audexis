use super::*;

pub(in crate::ui::hooks) fn provide_library(services: Arc<AppServices>, runtime: Handle) {
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
    let artwork_revision = use_state(|| 0);
    let connection = use_state(|| {
        services
            .cloud_context()
            .ok()
            .map(|context| context.connection_id)
    });
    let refresh_generation = use_state(|| 0);
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
        artwork_revision,
        connection,
        refresh_generation,
        requests: sender,
    });
    let loader_store = store.clone();
    let cloud_services = services.clone();
    let db = services.db.clone();
    let artwork = services.artwork.clone();
    let events_runtime = runtime.clone();
    use_future(move || {
        let receiver = receiver.take();
        let mut store = loader_store.clone();
        let db = db.clone();
        let runtime = runtime.clone();
        let cloud_services = cloud_services.clone();
        async move {
            let Some(mut receiver) = receiver else {
                return;
            };
            load(&mut store, &db, &runtime, Request::Discovery).await;
            while let Some(request) = receiver.recv().await {
                if matches!(request, Request::Refresh) {
                    let generation = *store.refresh_generation.peek();
                    if let Ok(context) = cloud_services.cloud_context() {
                        let _ = runtime
                            .spawn(async move { context.refresh_library().await })
                            .await;
                    }
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
                    store.refresh_pending.set(false);
                    if *store.refresh_generation.peek() != generation {
                        store.refresh();
                    }
                } else {
                    load(&mut store, &db, &runtime, request).await;
                }
            }
        }
    });
    let changes_store = store.clone();
    let changes_services = services.clone();
    let mut changes = use_hook(|| State::create(Some(services.library_files.subscribe())));
    use_future(move || {
        let receiver = changes.take();
        let mut store = changes_store.clone();
        let services = changes_services.clone();
        let artwork = artwork.clone();
        let runtime = events_runtime.clone();
        async move {
            let Some(mut receiver) = receiver else {
                return;
            };
            loop {
                let first = match receiver.recv().await {
                    Ok(change) => change,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        store.refresh();
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                };
                let connection = services
                    .cloud_context()
                    .ok()
                    .map(|context| context.connection_id);
                store.connection.set_if_modified(connection);
                let mut ids: std::collections::HashSet<_> = first.file_ids.into_iter().collect();
                let mut artwork_only = first.artwork_only;
                let _ = runtime
                    .spawn(async {
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await
                    })
                    .await;
                loop {
                    match receiver.try_recv() {
                        Ok(change) => {
                            ids.extend(change.file_ids);
                            artwork_only &= change.artwork_only;
                        }
                        Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {
                            artwork_only = false
                        }
                        Err(_) => break,
                    }
                }
                if !ids.is_empty() {
                    artwork
                        .invalidate_files(&ids.into_iter().collect::<Vec<_>>())
                        .await;
                    let next = store.artwork_revision.peek().wrapping_add(1);
                    store.artwork_revision.set(next);
                }
                if !artwork_only {
                    store.refresh();
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
