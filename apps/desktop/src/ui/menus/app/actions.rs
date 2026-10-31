use super::*;

pub(crate) async fn dispatch(
    action: AppAction,
    playback: &Playback,
    services: Arc<AppServices>,
    runtime: tokio::runtime::Handle,
    router: Option<RouterContext>,
) -> Result<(), String> {
    match action {
        AppAction::ExpandedPlayer => crate::ui::hooks::request_expanded_player(),
        AppAction::MiniPlayer => crate::ui::windows::player::open(services, runtime, true, None),
        AppAction::LibrarySettings | AppAction::Logs => {
            crate::ui::settings::request_section(if action == AppAction::Logs {
                crate::ui::settings::Section::Logs
            } else {
                crate::ui::settings::Section::Library
            });
            crate::ui::launch_settings(services, runtime);
        }
        AppAction::Settings => crate::ui::launch_settings(services, runtime),
        AppAction::NewPlaylist => {
            let _ = Platform::get()
                .launch_window(
                    WindowConfig::new_app(NewPlaylist { services, runtime })
                        .with_title("New Playlist")
                        .with_size(420., 210.),
                )
                .await;
        }
        AppAction::Import => {
            if let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await {
                let watcher = services
                    .watcher
                    .clone()
                    .ok_or("Library watcher unavailable")?;
                let path = folder.path().to_owned();
                runtime
                    .spawn(
                        async move { watcher.add_folder(&path).await.map_err(|e| e.to_string()) },
                    )
                    .await
                    .map_err(|e| e.to_string())??;
                services.library_files.publish_changed(vec![]);
            }
        }
        AppAction::Rescan => services
            .watcher
            .as_ref()
            .ok_or("Library watcher unavailable")?
            .rescan()
            .map_err(|e| e.to_string())?,
        AppAction::Missing => {
            router
                .ok_or("Navigation unavailable")?
                .push(NavigationTarget::<String>::Internal(
                    Route::Missing.to_string(),
                ))
                .map_err(|e| e.to_string())?;
        }
        AppAction::Queue => playback.toggle_panel(PlaybackPanelView::Queue),
        AppAction::Lyrics => playback.toggle_panel(PlaybackPanelView::Lyrics),
        AppAction::Play => playback.send(PlayerCmd::Play { resuming: true }),
        AppAction::Pause => playback.send(PlayerCmd::Pause),
        AppAction::Previous => playback.send(PlayerCmd::Previous),
        AppAction::Next => playback.send(PlayerCmd::Next),
        AppAction::Shuffle => playback.send(PlayerCmd::ToggleShuffle),
        AppAction::Repeat(mode) => {
            let order = [RepeatMode::Off, RepeatMode::Queue, RepeatMode::Track];
            let current = order
                .iter()
                .position(|m| *m == playback.state.peek().repeat_mode)
                .unwrap();
            let desired = order.iter().position(|m| *m == mode).unwrap();
            for _ in 0..(desired + 3 - current) % 3 {
                playback.send(PlayerCmd::CycleRepeat);
            }
        }
        AppAction::Cut | AppAction::Copy | AppAction::Paste | AppAction::SelectAll => {
            let (key, code) = match action {
                AppAction::Cut => ("x", Code::KeyX),
                AppAction::Copy => ("c", Code::KeyC),
                AppAction::Paste => ("v", Code::KeyV),
                _ => ("a", Code::KeyA),
            };
            let _ = Platform::get().post_callback(move |_, context| {
                use freya_winit::renderer::{NativeWindowEvent, NativeWindowEventAction};
                if let Some((&window_id, _)) =
                    context.windows.iter().find(|(_, w)| w.window().has_focus())
                {
                    for name in [KeyboardEventName::KeyDown, KeyboardEventName::KeyUp] {
                        let _ = context
                            .proxy
                            .send_event(NativeEvent::Window(NativeWindowEvent {
                                window_id,
                                action: NativeWindowEventAction::PlatformEvent(
                                    PlatformEvent::Keyboard {
                                        name,
                                        key: Key::Character(key.into()),
                                        code,
                                        modifiers: if name == KeyboardEventName::KeyUp {
                                            Modifiers::empty()
                                        } else if cfg!(target_os = "macos") {
                                            Modifiers::META
                                        } else {
                                            Modifiers::CONTROL
                                        },
                                    },
                                ),
                            }));
                    }
                }
            });
        }
        AppAction::Issue => {
            #[cfg(target_os = "macos")]
            let mut command = std::process::Command::new("open");
            #[cfg(target_os = "windows")]
            let mut command = {
                let mut c = std::process::Command::new("rundll32");
                c.arg("url.dll,FileProtocolHandler");
                c
            };
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            let mut command = std::process::Command::new("xdg-open");
            command
                .arg("https://github.com/adeyinkakp/audexis/issues")
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
