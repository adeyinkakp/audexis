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
        Action::Download | Action::RemoveDownload => {
            let context = services.cloud_context()?;
            let paths = tracks(target);
            runtime
                .spawn(async move {
                    for (_, path) in paths {
                        if path.starts_with("cloud-library://") {
                            let track = context
                                .library_track(&path)
                                .await
                                .map_err(|e| e.to_string())?;
                            if matches!(action, Action::Download) {
                                context.download(&track).await.map_err(|e| e.to_string())?;
                            } else {
                                context
                                    .remove_download(&track)
                                    .await
                                    .map_err(|e| e.to_string())?;
                            }
                        }
                    }
                    Ok::<_, String>(())
                })
                .await
                .map_err(|e| e.to_string())??;
            services.library_files.publish_changed(Vec::new());
            None
        }
        Action::Play => {
            target.play(playback);
            None
        }
        Action::Next => target.enqueue_command(true),
        Action::Last => target.enqueue_command(false),
        Action::Copy | Action::Reveal => {
            let ids: Vec<_> = tracks(target).into_iter().map(|(id, _)| id).collect();
            let db = services.db.clone();
            let paths = runtime
                .spawn(async move { db.library_disk_paths(&ids).await })
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            if paths.is_empty() {
                return Err("This song is not downloaded on this device.".into());
            }
            if matches!(action, Action::Copy) {
                Clipboard::set(paths.values().cloned().collect::<Vec<_>>().join("\n"))
                    .map_err(|e| format!("Could not copy paths: {e:?}"))?;
            } else {
                runtime
                    .spawn_blocking(move || reveal_files(paths.into_values().collect()))
                    .await
                    .map_err(|e| e.to_string())??;
            }
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
            services.sync_edits();
            None
        }
    };
    if let Some(command) = command {
        playback.send(command);
    }
    Ok(())
}

fn reveal_files(paths: Vec<String>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open")
            .arg("-R")
            .args(paths)
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("Could not show the selected files in Finder".into());
        }
    }
    #[cfg(target_os = "windows")]
    for path in paths {
        std::process::Command::new("explorer")
            .arg(format!("/select,{path}"))
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    for path in paths {
        if let Some(folder) = std::path::Path::new(&path).parent() {
            open::that(folder).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
