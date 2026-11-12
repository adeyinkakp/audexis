use super::*;

pub(crate) async fn run(
    action: ShortcutAction,
    playback: &crate::ui::hooks::Playback,
    services: Arc<AppServices>,
    runtime: tokio::runtime::Handle,
    router: Option<RouterContext>,
    search: Option<AccessibilityId>,
) -> Result<(), String> {
    let state = playback.snapshot();
    let app_action = match action {
        ShortcutAction::Settings => Some(AppAction::Settings),
        ShortcutAction::NewPlaylist => Some(AppAction::NewPlaylist),
        ShortcutAction::MiniPlayer => Some(AppAction::MiniPlayer),
        _ => None,
    };
    if let Some(action) = app_action {
        return app::dispatch(action, playback, services, runtime, router).await;
    }
    let route = match action {
        ShortcutAction::Search => Some(Route::Search),
        ShortcutAction::Songs => Some(Route::Songs),
        ShortcutAction::Albums => Some(Route::Albums),
        ShortcutAction::Artists => Some(Route::Artists),
        ShortcutAction::Playlists => Some(Route::Playlists),
        ShortcutAction::Favorites => Some(Route::Favorites),
        _ => None,
    };
    if let Some(route) = route {
        if let Some(router) = router {
            Platform::get().with_window(None, |window| {
                window.set_minimized(false);
                window.focus_window();
            });
            let mut expanded = playback.expanded;
            expanded.set(false);
            router
                .push(NavigationTarget::<String>::Internal(route.to_string()))
                .map_err(|e| e.to_string())?;
            if action == ShortcutAction::Search
                && let Some(search) = search
            {
                search.request_focus();
            }
        }
        return Ok(());
    }
    match action {
        ShortcutAction::ExpandedPlayer => {
            let mut expanded = playback.expanded;
            if *expanded.peek() {
                expanded.set(false);
            } else {
                crate::ui::hooks::request_expanded_player();
            }
        }
        ShortcutAction::Queue => playback.toggle_panel(crate::ui::hooks::PlaybackPanelView::Queue),
        ShortcutAction::Lyrics => {
            playback.toggle_panel(crate::ui::hooks::PlaybackPanelView::Lyrics)
        }
        _ if !playback.available() || state.path.is_none() => {}
        ShortcutAction::PlayPause => playback.send(if state.paused {
            PlayerCmd::Play { resuming: true }
        } else {
            PlayerCmd::Pause
        }),
        ShortcutAction::NextTrack => playback.send(PlayerCmd::Next),
        ShortcutAction::PreviousTrack => playback.send(PlayerCmd::Previous),
        ShortcutAction::SeekForward => playback.send(PlayerCmd::SeekMilliseconds {
            milliseconds: state
                .current_position
                .saturating_add(5000)
                .min(state.duration),
        }),
        ShortcutAction::SeekBackward => playback.send(PlayerCmd::SeekMilliseconds {
            milliseconds: state.current_position.saturating_sub(5000),
        }),
        ShortcutAction::Shuffle => playback.send(PlayerCmd::ToggleShuffle),
        ShortcutAction::Repeat => playback.send(PlayerCmd::CycleRepeat),
        ShortcutAction::Favorite => {
            let id = state.id;
            let db = services.db.clone();
            runtime
                .spawn(async move {
                    let loved: bool = sqlx::query_scalar(
                        "SELECT COALESCE((SELECT loved FROM media_info WHERE file_id=?),0)",
                    )
                    .bind(id)
                    .fetch_one(&db.pool)
                    .await
                    .map_err(|e| e.to_string())?;
                    db.favorite_selection(&[id], !loved)
                        .await
                        .map_err(|e| e.to_string())?;
                    services.library_files.publish_changed(vec![id]);
                    services.sync_edits();
                    Ok::<_, String>(())
                })
                .await
                .map_err(|e| e.to_string())??;
        }
        _ => {}
    }
    Ok(())
}
