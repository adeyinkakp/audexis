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
        menu.append(native_item(entry, &mut actions, None)?.as_ref())
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
    keyboard: Option<&crate::ui::keyboard::KeyboardSettings>,
) -> Result<Box<dyn muda::IsMenuItem>, String> {
    if entry.title.is_empty() {
        return Ok(Box::new(muda::PredefinedMenuItem::separator()));
    }
    if !entry.children.is_empty() {
        let group = muda::Submenu::new(entry.title, entry.enabled);
        for child in entry.children {
            group
                .append(native_item(child, actions, keyboard)?.as_ref())
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
        _ => None,
    };
    let fixed_accelerator = key.map(|key| {
        Accelerator::new(
            Some(if cfg!(target_os = "macos") {
                MenuModifiers::SUPER
            } else {
                MenuModifiers::CONTROL
            }),
            key,
        )
    });
    let shortcut = entry.action.and_then(|action| match action {
        Action::App(action) => crate::ui::keyboard::app_shortcut(action),
        _ => None,
    });
    let binding = shortcut.and_then(|action| {
        keyboard.and_then(|settings| settings.binding(action, cfg!(target_os = "macos")))
    });
    let configured = binding.as_ref().and_then(native_accelerator);
    let active = if crate::ui::keyboard::is_recording() {
        None
    } else {
        configured
    };
    let accelerator = active.or(fixed_accelerator);
    let title = if configured.is_none() {
        binding
            .as_ref()
            .map(|binding| format!("{}    {}", entry.title, binding.label()))
            .unwrap_or_else(|| entry.title.clone())
    } else {
        entry.title.clone()
    };
    let item: Box<dyn muda::IsMenuItem> = if let Some(checked) = entry.checked {
        let item = muda::CheckMenuItem::new(title, entry.enabled, checked, accelerator);
        if let (Some(action), Some(binding), Some(accelerator)) = (shortcut, binding, configured) {
            let target = item.clone();
            register_shortcut(
                action,
                binding,
                accelerator,
                entry.enabled,
                Box::new(move |value| {
                    let _ = target.set_accelerator(value);
                }),
            );
        }
        Box::new(item)
    } else {
        let item = muda::MenuItem::new(title, entry.enabled, accelerator);
        if let (Some(action), Some(binding), Some(accelerator)) = (shortcut, binding, configured) {
            let target = item.clone();
            register_shortcut(
                action,
                binding,
                accelerator,
                entry.enabled,
                Box::new(move |value| {
                    let _ = target.set_accelerator(value);
                }),
            );
        }
        Box::new(item)
    };
    if entry.enabled {
        if let Some(action) = entry.action {
            actions.push((item.id().clone(), action));
        }
    }
    Ok(item)
}

struct NativeShortcut {
    action: crate::ui::keyboard::ShortcutAction,
    binding: crate::ui::keyboard::KeyBinding,
    accelerator: muda::accelerator::Accelerator,
    enabled: bool,
    update: Box<dyn Fn(Option<muda::accelerator::Accelerator>)>,
}
thread_local! {
    static SHORTCUTS: std::cell::RefCell<Vec<NativeShortcut>> = const { std::cell::RefCell::new(Vec::new()) };
}
pub(super) fn clear_shortcuts() {
    SHORTCUTS.with(|items| items.borrow_mut().clear());
}
fn register_shortcut(
    action: crate::ui::keyboard::ShortcutAction,
    binding: crate::ui::keyboard::KeyBinding,
    accelerator: muda::accelerator::Accelerator,
    enabled: bool,
    update: Box<dyn Fn(Option<muda::accelerator::Accelerator>)>,
) {
    SHORTCUTS.with(|items| {
        items.borrow_mut().push(NativeShortcut {
            action,
            binding,
            accelerator,
            enabled,
            update,
        })
    });
}
pub(crate) fn enable_shortcuts(enabled: bool) {
    #[cfg(target_os = "macos")]
    if enabled {
        restore_cocoa_shortcuts();
    }
    SHORTCUTS.with(|items| {
        for item in items.borrow().iter() {
            (item.update)(enabled.then_some(item.accelerator));
        }
    });
    #[cfg(target_os = "macos")]
    if !enabled {
        suspend_cocoa_shortcuts();
    }
}
pub(crate) fn handles_shortcut(
    action: crate::ui::keyboard::ShortcutAction,
    binding: &crate::ui::keyboard::KeyBinding,
) -> bool {
    !crate::ui::keyboard::is_recording()
        && SHORTCUTS.with(|items| {
            items
                .borrow()
                .iter()
                .any(|item| item.enabled && item.action == action && &item.binding == binding)
        })
}
fn native_accelerator(
    binding: &crate::ui::keyboard::KeyBinding,
) -> Option<muda::accelerator::Accelerator> {
    if !binding.control && !binding.meta {
        return None;
    }
    let mut parts = Vec::new();
    if binding.control {
        parts.push("Control");
    }
    if binding.meta {
        parts.push("Super");
    }
    if binding.alt {
        parts.push("Alt");
    }
    if binding.shift {
        parts.push("Shift");
    }
    parts.push(&binding.key);
    parts.join("+").parse().ok()
}

#[cfg(target_os = "macos")]
struct SuspendedEquivalent {
    item: objc2::rc::Retained<objc2_app_kit::NSMenuItem>,
    key: objc2::rc::Retained<objc2_foundation::NSString>,
    modifiers: objc2_app_kit::NSEventModifierFlags,
}
#[cfg(target_os = "macos")]
thread_local! {
    static SUSPENDED_EQUIVALENTS: std::cell::RefCell<Vec<SuspendedEquivalent>> = const { std::cell::RefCell::new(Vec::new()) };
}
#[cfg(target_os = "macos")]
fn suspend_menu_keys(menu: &objc2_app_kit::NSMenu, saved: &mut Vec<SuspendedEquivalent>) {
    for item in menu.itemArray().to_vec() {
        let key = item.keyEquivalent();
        if !key.is_empty() {
            let modifiers = item.keyEquivalentModifierMask();
            item.setKeyEquivalent(&objc2_foundation::NSString::from_str(""));
            saved.push(SuspendedEquivalent {
                item: item.clone(),
                key,
                modifiers,
            });
        }
        if let Some(submenu) = item.submenu() {
            suspend_menu_keys(&submenu, saved);
        }
    }
}
#[cfg(target_os = "macos")]
pub(super) fn suspend_cocoa_shortcuts() {
    let Some(main) = objc2::MainThreadMarker::new() else {
        return;
    };
    if let Some(menu) = objc2_app_kit::NSApplication::sharedApplication(main).mainMenu() {
        SUSPENDED_EQUIVALENTS.with(|saved| suspend_menu_keys(&menu, &mut saved.borrow_mut()));
    }
}
#[cfg(target_os = "macos")]
fn restore_menu_keys(saved: &mut Vec<SuspendedEquivalent>) {
    for entry in saved.drain(..) {
        entry.item.setKeyEquivalent(&entry.key);
        entry.item.setKeyEquivalentModifierMask(entry.modifiers);
    }
}
#[cfg(target_os = "macos")]
fn restore_cocoa_shortcuts() {
    SUSPENDED_EQUIVALENTS.with(|saved| restore_menu_keys(&mut saved.borrow_mut()));
}
