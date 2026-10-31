use super::*;

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(super) fn native_popup(
    window: &freya::winit::window::Window,
    entries: Vec<Entry>,
) -> Result<Option<Action>, String> {
    use muda::ContextMenu as _;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let menu = muda::Menu::new();
    let mut actions = Vec::new();
    for entry in entries {
        menu.append(native_item(entry, &mut actions)?.as_ref())
            .map_err(|e| e.to_string())?;
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

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(super) fn native_item(
    entry: Entry,
    actions: &mut Vec<(muda::MenuId, Action)>,
) -> Result<Box<dyn muda::IsMenuItem>, String> {
    if entry.title.is_empty() {
        return Ok(Box::new(muda::PredefinedMenuItem::separator()));
    }
    if !entry.children.is_empty() {
        let group = muda::Submenu::new(entry.title, entry.enabled);
        for child in entry.children {
            group
                .append(native_item(child, actions)?.as_ref())
                .map_err(|e| e.to_string())?;
        }
        return Ok(Box::new(group));
    }
    use muda::accelerator::{Accelerator, Code as MenuCode, Modifiers as MenuModifiers};
    let key = match entry.action {
        Some(Action::App(app::AppAction::Cut)) => Some(MenuCode::KeyX),
        Some(Action::App(app::AppAction::Copy)) => Some(MenuCode::KeyC),
        Some(Action::App(app::AppAction::Paste)) => Some(MenuCode::KeyV),
        Some(Action::App(app::AppAction::SelectAll)) => Some(MenuCode::KeyA),
        Some(Action::App(app::AppAction::Settings)) => Some(MenuCode::Comma),
        Some(Action::App(app::AppAction::NewPlaylist)) => Some(MenuCode::KeyN),
        _ => None,
    };
    let accelerator = key.map(|key| {
        Accelerator::new(
            Some(if cfg!(target_os = "macos") {
                MenuModifiers::SUPER
            } else {
                MenuModifiers::CONTROL
            }),
            key,
        )
    });
    let item: Box<dyn muda::IsMenuItem> = if let Some(checked) = entry.checked {
        Box::new(muda::CheckMenuItem::new(
            entry.title,
            entry.enabled,
            checked,
            accelerator,
        ))
    } else {
        Box::new(muda::MenuItem::new(entry.title, entry.enabled, accelerator))
    };
    if entry.enabled {
        if let Some(action) = entry.action {
            actions.push((item.id().clone(), action));
        }
    }
    Ok(item)
}
