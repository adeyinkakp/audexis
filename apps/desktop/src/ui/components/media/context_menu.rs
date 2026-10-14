use super::MediaTarget;
use crate::ui::hooks::Playback;
use freya::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Action {
    Play,
    Next,
    Last,
    Remove,
    Copy,
}
struct Entry {
    title: &'static str,
    action: Option<Action>,
    enabled: bool,
}
fn entries(target: &MediaTarget, available: bool) -> Vec<Entry> {
    let can_queue = available && target.enqueue_command(false).is_some();
    let mut items = vec![
        Entry {
            title: "Play",
            action: Some(Action::Play),
            enabled: available && target.play_command().is_some(),
        },
        Entry {
            title: "Play Next",
            action: Some(Action::Next),
            enabled: can_queue,
        },
        Entry {
            title: "Play Last",
            action: Some(Action::Last),
            enabled: can_queue,
        },
        Entry {
            title: "",
            action: None,
            enabled: false,
        },
        Entry {
            title: "Get Info",
            action: None,
            enabled: false,
        },
        Entry {
            title: "Edit Metadata Fields…",
            action: None,
            enabled: false,
        },
        Entry {
            title: "Heart",
            action: None,
            enabled: false,
        },
        Entry {
            title: "Add to Playlist",
            action: None,
            enabled: false,
        },
        Entry {
            title: "",
            action: None,
            enabled: false,
        },
        Entry {
            title: "Show Album in Library",
            action: None,
            enabled: false,
        },
        Entry {
            title: if matches!(target, MediaTarget::Album { .. }) {
                "Copy file paths"
            } else {
                "Copy file path"
            },
            action: Some(Action::Copy),
            enabled: !target.paths().is_empty(),
        },
    ];
    if matches!(
        target,
        MediaTarget::Track {
            queue_id: Some(_),
            ..
        }
    ) {
        items.push(Entry {
            title: "",
            action: None,
            enabled: false,
        });
        items.push(Entry {
            title: "Remove from Queue",
            action: Some(Action::Remove),
            enabled: available,
        });
    }
    items
}
fn dispatch(action: Action, target: &MediaTarget, playback: &Playback) {
    let command = match action {
        Action::Play => {
            target.play(playback);
            None
        }
        Action::Next => target.enqueue_command(true),
        Action::Last => target.enqueue_command(false),
        Action::Remove => match target {
            MediaTarget::Track {
                queue_id: Some(queue_id),
                ..
            } => Some(audexis_audio::PlayerCmd::RemoveQueueEntry {
                queue_id: queue_id.clone(),
            }),
            _ => None,
        },
        Action::Copy => {
            if let Err(e) = Clipboard::set(target.paths()) {
                let mut error = playback.error;
                error.set(Some(format!("Could not copy path: {e:?}")));
            }
            None
        }
    };
    if let Some(command) = command {
        playback.send(command);
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(super) fn open(_event: &Event<PressEventData>, target: MediaTarget, playback: Playback) {
    let items = entries(&target, playback.available());
    let (tx, rx) = tokio::sync::oneshot::channel();

    Platform::get().with_window(None, move |window| {
        let _ = tx.send(native_popup(window, items));
    });
    spawn(async move {
        match rx.await {
            Ok(Ok(Some(action))) => dispatch(action, &target, &playback),
            Ok(Ok(None)) => {}
            result => {
                let mut error = playback.error;
                error.set(Some(format!("Could not open context menu: {result:?}")));
            }
        }
    });
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn native_popup(
    window: &freya::winit::window::Window,
    entries: Vec<Entry>,
) -> Result<Option<Action>, String> {
    use muda::ContextMenu as _;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let menu = muda::Menu::new();
    let mut actions = Vec::new();
    for entry in entries {
        if entry.title.is_empty() {
            menu.append(&muda::PredefinedMenuItem::separator())
                .map_err(|e| e.to_string())?;
        } else {
            let item = muda::MenuItem::new(entry.title, entry.enabled, None);
            if entry.enabled {
                if let Some(action) = entry.action {
                    actions.push((item.id().clone(), action));
                }
            }
            menu.append(&item).map_err(|e| e.to_string())?;
        }
    }
    let handle = window.window_handle().map_err(|e| e.to_string())?;

    let selected = unsafe {
        match handle.as_raw() {
            #[cfg(target_os = "macos")]
            RawWindowHandle::AppKit(handle) => {
                menu.show_context_menu_for_nsview(handle.ns_view.as_ptr(), None)
            }
            #[cfg(target_os = "windows")]
            RawWindowHandle::Win32(handle) => {
                menu.show_context_menu_for_hwnd(handle.hwnd.get(), None)
            }
            _ => return Err("Unsupported native window handle".into()),
        }
    };
    let action = muda::MenuEvent::receiver().try_iter().find_map(|event| {
        actions
            .iter()
            .find(|(id, _)| id == &event.id)
            .map(|(_, action)| *action)
    });
    Ok(if selected { action } else { None })
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(super) fn open(event: &Event<PressEventData>, target: MediaTarget, playback: Playback) {
    let mut menu = Menu::new();
    for entry in entries(&target, playback.available()) {
        if entry.title.is_empty() {
            menu = menu.child(
                rect()
                    .width(Size::fill())
                    .height(Size::px(1.0))
                    .background((100, 100, 100)),
            );
            continue;
        }
        if !entry.enabled {
            menu = menu.child(
                rect()
                    .padding(8.0)
                    .opacity(0.45)
                    .child(label().text(entry.title)),
            );
            continue;
        }
        let target = target.clone();
        let playback = playback.clone();
        menu = menu.child(MenuButton::new().child(entry.title).on_press(move |_| {
            ContextMenu::close();
            if entry.enabled {
                if let Some(action) = entry.action {
                    dispatch(action, &target, &playback);
                }
            }
        }));
    }
    ContextMenu::open_from_event(event, menu);
}
