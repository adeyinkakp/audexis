use super::*;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(super) fn fallback_item(
    entry: Entry,
    target: MediaTarget,
    playback: Playback,
    services: Arc<AppServices>,
    runtime: tokio::runtime::Handle,
    router: Option<RouterContext>,
    playlist: Option<PlaylistOccurrence>,
    mut busy: State<bool>,
) -> Element {
    if entry.title.is_empty() {
        return rect()
            .height(Size::px(1.))
            .width(Size::fill())
            .background((100, 100, 100))
            .into_element();
    }
    if !entry.enabled {
        return rect()
            .padding(8.)
            .opacity(0.45)
            .child(entry.title)
            .into_element();
    }
    if !entry.children.is_empty() {
        return SubMenu::new()
            .label(entry.title)
            .children(entry.children.into_iter().map(|child| {
                fallback_item(
                    child,
                    target.clone(),
                    playback.clone(),
                    services.clone(),
                    runtime.clone(),
                    router.clone(),
                    playlist,
                    busy,
                )
            }))
            .into_element();
    }
    let title = format!(
        "{}{}",
        if entry.checked == Some(true) {
            "✓ "
        } else {
            ""
        },
        entry.title
    );
    MenuButton::new()
        .child(title)
        .on_press(move |_| {
            ContextMenu::close();
            if *busy.peek() {
                return;
            }
            busy.set(true);
            let target = target.clone();
            let playback = playback.clone();
            let services = services.clone();
            let runtime = runtime.clone();
            let router = router.clone();
            spawn(async move {
                if let Some(action) = entry.action {
                    if let Err(message) = dispatch(
                        action, &target, &playback, services, runtime, router, playlist,
                    )
                    .await
                    {
                        let mut error = playback.error;
                        error.set(Some(message));
                    }
                }
                busy.set(false);
            });
        })
        .into_element()
}
