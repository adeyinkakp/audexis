use super::*;

pub(crate) fn open(
    event: &Event<PressEventData>,
    target: MediaTarget,
    playback: Playback,
    services: Arc<AppServices>,
    runtime: tokio::runtime::Handle,
    router: Option<RouterContext>,
    playlist: Option<PlaylistOccurrence>,
    mut busy: State<bool>,
) {
    if *busy.peek() {
        return;
    }
    busy.set(true);
    let platform = Platform::get();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let event = event.clone();
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let _ = event;
    spawn(async move {
        let ids = tracks(&target).into_iter().map(|t| t.0).collect::<Vec<_>>();
        let db = services.db.clone();
        let state = runtime
            .spawn(async move { db.media_menu_state(&ids).await.map_err(|e| e.to_string()) })
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r);
        let items = entries(
            &target,
            playback.available(),
            &state,
            playlist,
            router.is_some(),
        );
        if let Err(message) = &state {
            let mut error = playback.error;
            error.set(Some(format!("Could not load menu options: {message}")));
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let (tx, rx) = tokio::sync::oneshot::channel();
            platform.with_window(None, move |window| {
                let _ = tx.send(native_popup(window, items));
            });
            let result = match rx.await {
                Ok(Ok(Some(action))) => {
                    dispatch(
                        action, &target, &playback, services, runtime, router, playlist,
                    )
                    .await
                }
                Ok(Ok(None)) => Ok(()),
                other => Err(format!("Could not open context menu: {other:?}")),
            };
            if let Err(message) = result {
                let mut error = playback.error;
                error.set(Some(message));
            }
            busy.set(false);
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = platform;
            let menu = Menu::new().children(items.into_iter().map(|entry| {
                fallback_item(
                    entry,
                    target.clone(),
                    playback.clone(),
                    services.clone(),
                    runtime.clone(),
                    router.clone(),
                    playlist,
                    busy,
                )
            }));
            ContextMenu::open_from_event(&event, menu);
            busy.set(false);
        }
    });
}
