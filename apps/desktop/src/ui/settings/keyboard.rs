use crate::{
    services::AppServices,
    ui::{
        keyboard::{self, KeyBinding, KeyboardSettings, ShortcutAction},
        theme::use_audexis_colors,
    },
};
use freya::prelude::*;
use std::sync::Arc;

#[derive(PartialEq)]
pub(super) struct KeyboardPanel;
impl Component for KeyboardPanel {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let mut settings = keyboard::use_keyboard_settings();
        let mut recording = use_state(|| None::<ShortcutAction>);
        let return_focus = use_state(|| None::<AccessibilityId>);
        let mut error = use_state(|| None::<(ShortcutAction, String)>);
        let mut reset_error = use_state(|| None::<String>);
        ScrollView::new().width(Size::fill()).height(Size::fill()).child(
            rect().width(Size::fill()).spacing(16.)
                .child(label().text("Keyboard").font_size(20.).font_weight(FontWeight::BOLD))
                .child(label().text("Change a shortcut by selecting its key combination and pressing new keys. Escape cancels recording. Changes are saved automatically.").font_size(13.).color(colors.muted_foreground))
                .child(label().text("Tab moves between controls. Use arrow keys to browse songs, Shift + arrows to extend a selection, Enter to play, and Shift + F10 to open song options.").font_size(12.).color(colors.muted_foreground))
                .child(Button::new().outline().child("Restore all defaults").on_press(move|_|{
                    match services.update_config(|c|c.keyboard=KeyboardSettings::default()) {
                        Ok(())=>{settings.set(KeyboardSettings::default());recording.set(None);error.set(None);reset_error.set(None);},
                        Err(message)=>reset_error.set(Some(message.to_string())),
                    }
                }))
                .maybe_child(reset_error.read().clone().map(|message|label().text(message).color(colors.destructive)))
                .children(ShortcutAction::ALL.into_iter().map(|action|ShortcutRow{action,settings,recording,return_focus,error}.into_element()))
        )
    }
}

fn save(
    services: &AppServices,
    action: ShortcutAction,
    binding: Option<KeyBinding>,
) -> Result<KeyboardSettings, String> {
    let mut next = services
        .user_config
        .lock()
        .map_err(|_| "Settings are unavailable")?
        .keyboard
        .clone();
    next.assign(action, binding, cfg!(target_os = "macos"))?;
    services
        .update_config(|c| c.keyboard = next.clone())
        .map_err(|e| e.to_string())?;
    Ok(next)
}

#[derive(PartialEq)]
struct ShortcutRow {
    action: ShortcutAction,
    settings: State<KeyboardSettings>,
    recording: State<Option<ShortcutAction>>,
    return_focus: State<Option<AccessibilityId>>,
    error: State<Option<(ShortcutAction, String)>>,
}
impl Component for ShortcutRow {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let action = self.action;
        let mut settings = self.settings;
        let mut recording = self.recording;
        let mut return_focus = self.return_focus;
        let mut error = self.error;
        let active = *recording.read() == Some(action);
        let binding = settings.read().binding(action, cfg!(target_os = "macos"));
        let clear_services = services.clone();
        let reset_services = services.clone();
        let mut row = rect()
            .width(Size::fill())
            .padding(12.)
            .spacing(10.)
            .corner_radius(10.)
            .background(colors.surface_subtle)
            .child(
                rect()
                    .width(Size::fill())
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .spacing(8.)
                    .child(
                        label()
                            .text(action.label())
                            .width(Size::flex(1.))
                            .font_size(13.),
                    )
                    .child(
                        Button::new()
                            .outline()
                            .child(
                                label()
                                    .text(if active {
                                        "Recording…".into()
                                    } else {
                                        binding
                                            .as_ref()
                                            .map(KeyBinding::label)
                                            .unwrap_or("Not assigned".into())
                                    })
                                    .font_size(12.),
                            )
                            .on_press(move |_| {
                                return_focus
                                    .set(Some(*Platform::get().focused_accessibility_id.peek()));
                                recording.set(Some(action));
                                error.set(None);
                            }),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .compact()
                            .enabled(binding.is_some())
                            .child("Clear")
                            .on_press(move |_| match save(&clear_services, action, None) {
                                Ok(next) => {
                                    settings.set(next);
                                    recording.set(None);
                                    error.set(None);
                                }
                                Err(message) => error.set(Some((action, message))),
                            }),
                    )
                    .child(
                        Button::new().flat().compact().child("Reset").on_press(
                            move |_| match save(
                                &reset_services,
                                action,
                                action.default_binding(cfg!(target_os = "macos")),
                            ) {
                                Ok(next) => {
                                    settings.set(next);
                                    recording.set(None);
                                    error.set(None);
                                }
                                Err(message) => error.set(Some((action, message))),
                            },
                        ),
                    ),
            );
        if active {
            row = row.child(ShortcutRecorder {
                action,
                on_cancel: EventHandler::new(move |restore| {
                    recording.set(None);
                    error.set(None);
                    if restore && let Some(id) = *return_focus.peek() {
                        id.request_focus();
                    }
                }),
                on_save: EventHandler::new(move |binding| {
                    match save(&services, action, Some(binding)) {
                        Ok(next) => {
                            settings.set(next);
                            recording.set(None);
                            error.set(None);
                            if let Some(id) = *return_focus.peek() {
                                id.request_focus();
                            }
                        }
                        Err(message) => error.set(Some((action, message))),
                    }
                }),
            });
        }
        if let Some((failed, message)) = error.read().clone()
            && failed == action
        {
            row = row.child(
                label()
                    .text(message)
                    .font_size(12.)
                    .color(colors.destructive),
            );
        }
        row
    }
}

#[derive(PartialEq)]
struct ShortcutRecorder {
    action: ShortcutAction,
    on_save: EventHandler<KeyBinding>,
    on_cancel: EventHandler<bool>,
}
impl Component for ShortcutRecorder {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let id = use_a11y();
        let mut focused = use_state(|| false);
        let mut held = use_state(std::collections::HashSet::<Code>::new);
        let mut preview = use_state(|| "Press a key combination…".to_string());
        use_hook(|| keyboard::set_recording(true));
        use_drop(|| keyboard::set_recording(false));
        let save = self.on_save.clone();
        let cancel = self.on_cancel.clone();
        let cancel_button = cancel.clone();
        rect()
            .width(Size::fill())
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(8.)
            .on_global_key_down(move |event: Event<KeyboardEventData>| {
                event.stop_propagation();
                event.prevent_default();
                if matches!(event.key, Key::Named(NamedKey::Tab | NamedKey::Escape)) {
                    cancel.call(true);
                    return;
                }
                if !held.write().insert(event.code) { return; }
                if let Some(binding) = keyboard::event_binding(&event) {
                    preview.set(binding.label());
                    save.call(binding);
                } else {
                    preview.set(modifier_preview(event.modifiers));
                }
            })
            .on_global_key_up(move |event: Event<KeyboardEventData>| {
                held.write().remove(&event.code);
                if keyboard::event_binding(&event).is_none() {
                    preview.set(modifier_preview(event.modifiers));
                }
            })
            .child(
                rect()
                    .width(Size::flex(1.))
                    .padding(12.)
                    .corner_radius(8.)
                    .background(colors.background)
                    .border(Border::new().width(2.).fill(colors.primary))
                    .a11y_id(id)
                    .a11y_focusable(true)
                    .a11y_role(AccessibilityRole::Button)
                    .a11y_alt(format!("Recording shortcut for {}. Press a key combination, Escape to cancel, or Tab to leave.", self.action.label()))
                    .on_sized(move |_| {
                        if !*focused.peek() {
                            focused.set(true);
                            id.request_focus();
                        }
                    })
                    .child(label().text(preview.read().clone()).font_size(13.)),
            )
            .child(Button::new().flat().child("Cancel").on_press(move |_| cancel_button.call(true)))
    }
}

fn modifier_preview(modifiers: Modifiers) -> String {
    if !modifiers
        .intersects(Modifiers::CONTROL | Modifiers::META | Modifiers::ALT | Modifiers::SHIFT)
    {
        return "Press a key combination…".into();
    }
    KeyBinding {
        key: "…".into(),
        control: modifiers.contains(Modifiers::CONTROL),
        meta: modifiers.contains(Modifiers::META),
        alt: modifiers.contains(Modifiers::ALT),
        shift: modifiers.contains(Modifiers::SHIFT),
    }
    .label()
}
