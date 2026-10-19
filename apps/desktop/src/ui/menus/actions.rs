use super::*;

pub(super) async fn dispatch(
    action: Action,
    target: &MediaTarget,
    playback: &Playback,
    services: Arc<AppServices>,
    runtime: tokio::runtime::Handle,
    router: Option<RouterContext>,
    playlist: Option<PlaylistOccurrence>,
) -> Result<(), String> {
    let command = match action {
        Action::App(action) => {
            return app::dispatch(action, playback, services, runtime, router).await;
        }
        Action::Info | Action::Edit => {
            crate::ui::track_info::open(
                services,
                runtime,
                tracks(target),
                crate::ui::track_info::Tab::Details,
            );
            None
        }
        Action::Play => {
            target.play(playback);
            None
        }
        Action::Next => target.enqueue_command(true),
        Action::Last => target.enqueue_command(false),
        Action::Copy => {
            Clipboard::set(target.paths()).map_err(|e| format!("Could not copy paths: {e:?}"))?;
            None
        }
        Action::RemoveQueue => match target {
            MediaTarget::Selection { tracks } => {
                for track in tracks.iter() {
                    if let Some(id) = &track.queue_id {
                        playback.send(audexis_audio::PlayerCmd::RemoveQueueEntry {
                            queue_id: id.clone(),
                        });
                    }
                }
                None
            }
            MediaTarget::Track {
                queue_id: Some(id), ..
            } => Some(audexis_audio::PlayerCmd::RemoveQueueEntry {
                queue_id: id.clone(),
            }),
            _ => None,
        },
        Action::ShowAlbum => {
            let id = tracks(target)
                .first()
                .map(|t| t.0)
                .ok_or("No song selected")?;
            router
                .ok_or("Library navigation is unavailable")?
                .push(NavigationTarget::<String>::Internal(
                    Route::AlbumForTrack { file_id: id }.to_string(),
                ))
                .map_err(|e| e.to_string())?;
            None
        }
        Action::Heart(_) | Action::AddPlaylist(_) | Action::RemovePlaylist => {
            let ids = tracks(target)
                .into_iter()
                .map(|t| t.0)
                .filter(|id| *id > 0)
                .collect::<Vec<_>>();
            let occurrences = match target {
                MediaTarget::Selection { tracks } => tracks
                    .iter()
                    .filter_map(|track| track.playlist)
                    .collect::<Vec<_>>(),
                _ => playlist.into_iter().collect(),
            };
            let requested = ids.clone();
            let db = services.db.clone();
            runtime
                .spawn(async move {
                    match action {
                        Action::Heart(loved) => db
                            .favorite_selection(&requested, loved)
                            .await
                            .map_err(|e| e.to_string()),
                        Action::AddPlaylist(id) => db
                            .add_selection_to_playlist(id, &requested)
                            .await
                            .map_err(|e| e.to_string()),
                        Action::RemovePlaylist => {
                            if occurrences.is_empty() {
                                return Err("No playlist entries selected".into());
                            }
                            let entries = occurrences
                                .iter()
                                .map(|p| (p.playlist, p.ord, p.file))
                                .collect::<Vec<_>>();
                            db.remove_playlist_occurrences(&entries)
                                .await
                                .map_err(|e| e.to_string())
                        }
                        _ => unreachable!(),
                    }
                })
                .await
                .map_err(|e| e.to_string())??;
            services.library_files.publish_changed(ids);
            None
        }
    };
    if let Some(command) = command {
        playback.send(command);
    }
    Ok(())
}
