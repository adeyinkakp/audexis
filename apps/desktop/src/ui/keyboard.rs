use crate::{
    services::AppServices,
    ui::{
        hooks::use_playback,
        menus::app::{self, AppAction},
        routes::Route,
    },
};
use audexis_audio::PlayerCmd;
pub(crate) use audexis_core::config::keyboard::{KeyBinding, KeyboardSettings, ShortcutAction};
use freya::{
    prelude::*,
    router::{NavigationTarget, RouterContext},
};
use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

static RECORDING: AtomicUsize = AtomicUsize::new(0);
pub(crate) fn is_recording() -> bool {
    RECORDING.load(Ordering::Relaxed) > 0
}
pub(crate) fn set_recording(value: bool) {
    if value {
        RECORDING.fetch_add(1, Ordering::Relaxed);
    } else {
        RECORDING.fetch_sub(1, Ordering::Relaxed);
    }
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    crate::ui::menus::native::enable_shortcuts(!is_recording());
}

#[derive(Clone)]
pub(crate) struct ShortcutCommands(pub EventHandler<ShortcutAction>);

#[derive(Clone, Copy)]
pub(crate) struct SearchFocus(pub State<Option<AccessibilityId>>);

#[derive(PartialEq)]
pub(crate) struct KeyboardShortcuts {
    pub child: Element,
}
impl Component for KeyboardShortcuts {
    fn render(&self) -> impl IntoElement {
        let settings = use_keyboard_settings();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let playback = use_playback();
        let router = RouterContext::try_get();
        let search = use_state(|| None::<AccessibilityId>);
        use_provide_context(move || SearchFocus(search));
        let command_services = services.clone();
        let command_runtime = runtime.clone();
        let command_playback = playback.clone();
        let command_router = router.clone();
        let commands = EventHandler::new(move |action| {
            if is_recording() {
                return;
            }
            let services = command_services.clone();
            let runtime = command_runtime.clone();
            let playback = command_playback.clone();
            let router = command_router.clone();
            spawn(async move {
                if let Err(message) =
                    run(action, &playback, services, runtime, router, *search.peek()).await
                {
                    let mut error = playback.error;
                    error.set(Some(message));
                }
            });
        });
        let provided_commands = commands.clone();
        use_provide_context(move || ShortcutCommands(provided_commands));
        let mut held = use_state(HashSet::<Code>::new);
        let focused = *Platform::get().is_app_focused.read();
        use_side_effect_with_deps(&focused, move |_| held.write().clear());
        rect()
            .expanded()
            .content(Content::Flex)
            .on_global_key_up(move |event: Event<KeyboardEventData>| {
                held.write().remove(&event.code);
            })
            .on_global_key_down(move |event: Event<KeyboardEventData>| {
                if is_recording() || !*Platform::get().is_app_focused.peek() {
                    return;
                }
                let Some(action) = matching_action(&settings.peek(), &event) else {
                    return;
                };
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                if event_binding(&event).as_ref().is_some_and(|binding| {
                    crate::ui::menus::native::handles_shortcut(action, binding)
                }) {
                    return;
                }
                let modified = event
                    .modifiers
                    .intersects(Modifiers::CONTROL | Modifiers::META);
                let role = Platform::get().focused_accessibility_node.peek().role();
                if matches!(
                    role,
                    AccessibilityRole::TextInput
                        | AccessibilityRole::MultilineTextInput
                        | AccessibilityRole::SearchInput
                        | AccessibilityRole::EditableComboBox
                ) && !modified
                {
                    return;
                }
                if matches!(
                    role,
                    AccessibilityRole::Button
                        | AccessibilityRole::DefaultButton
                        | AccessibilityRole::CheckBox
                        | AccessibilityRole::RadioButton
                        | AccessibilityRole::Switch
                        | AccessibilityRole::Slider
                        | AccessibilityRole::MenuItem
                ) && !event
                    .modifiers
                    .intersects(Modifiers::CONTROL | Modifiers::META | Modifiers::ALT)
                    && event.is_press_event()
                {
                    return;
                }
                let repeatable = matches!(
                    action,
                    ShortcutAction::SeekForward | ShortcutAction::SeekBackward
                );
                if !held.write().insert(event.code) && !repeatable {
                    event.stop_propagation();
                    event.prevent_default();
                    return;
                }
                event.stop_propagation();
                event.prevent_default();
                commands.call(action);
            })
            .child(self.child.clone())
    }
}

mod bindings;
mod commands;
mod menu;

pub(crate) use bindings::{app_shortcut, event_binding, matching_action, use_keyboard_settings};
pub(crate) use commands::run;
pub(crate) use menu::KeyboardMenu;
