use super::*;

pub(crate) fn event_binding(event: &KeyboardEventData) -> Option<KeyBinding> {
    let mut key = match &event.key {
        Key::Character(text) if text == " " => "Space".to_string(),
        Key::Character(text) => text.to_lowercase(),
        Key::Named(
            NamedKey::Shift
            | NamedKey::Control
            | NamedKey::Alt
            | NamedKey::Meta
            | NamedKey::CapsLock,
        ) => return None,
        Key::Named(key) => format!("{key:?}"),
    };
    if event
        .modifiers
        .intersects(Modifiers::CONTROL | Modifiers::META | Modifiers::ALT)
    {
        let code = format!("{:?}", event.code);
        if let Some(letter) = code.strip_prefix("Key").filter(|key| key.len() == 1) {
            key = letter.to_ascii_lowercase();
        } else if let Some(digit) = code.strip_prefix("Digit").filter(|key| key.len() == 1) {
            key = digit.to_string();
        } else if let Some(symbol) = match event.code {
            Code::Comma => Some(","),
            Code::Period => Some("."),
            Code::Slash => Some("/"),
            Code::Semicolon => Some(";"),
            Code::Quote => Some("'"),
            Code::BracketLeft => Some("["),
            Code::BracketRight => Some("]"),
            Code::Backslash => Some("\\"),
            Code::Backquote => Some("`"),
            Code::Minus => Some("-"),
            Code::Equal => Some("="),
            _ => None,
        } {
            key = symbol.to_string();
        }
    }
    Some(KeyBinding {
        key,
        control: event.modifiers.contains(Modifiers::CONTROL),
        meta: event.modifiers.contains(Modifiers::META),
        shift: event.modifiers.contains(Modifiers::SHIFT),
        alt: event.modifiers.contains(Modifiers::ALT),
    })
}

pub(crate) fn use_keyboard_settings() -> State<KeyboardSettings> {
    let services = use_consume::<Arc<AppServices>>();
    let mut settings = use_state(|| {
        services
            .user_config
            .lock()
            .map(|c| c.keyboard.clone())
            .unwrap_or_default()
    });
    use_future(move || {
        let services = services.clone();
        let mut updates = services.config_updates.subscribe();
        async move {
            while !matches!(
                updates.recv().await,
                Err(tokio::sync::broadcast::error::RecvError::Closed)
            ) {
                if let Ok(config) = services.user_config.lock() {
                    settings.set_if_modified(config.keyboard.clone());
                }
            }
        }
    });
    settings
}

pub(crate) fn matching_action(
    settings: &KeyboardSettings,
    event: &KeyboardEventData,
) -> Option<ShortcutAction> {
    let binding = event_binding(event)?;
    let mut logical = binding.clone();
    if let Key::Character(text) = &event.key {
        logical.key = if text == " " {
            "Space".into()
        } else {
            text.to_lowercase()
        };
    }
    ShortcutAction::ALL.into_iter().find(|action| {
        settings
            .binding(*action, cfg!(target_os = "macos"))
            .as_ref()
            .is_some_and(|configured| configured == &binding || configured == &logical)
    })
}

pub(crate) fn app_shortcut(action: AppAction) -> Option<ShortcutAction> {
    use ShortcutAction as S;
    Some(match action {
        AppAction::Shortcut(action) => action,
        AppAction::Settings => S::Settings,
        AppAction::NewPlaylist => S::NewPlaylist,
        AppAction::Next => S::NextTrack,
        AppAction::Previous => S::PreviousTrack,
        AppAction::Queue => S::Queue,
        AppAction::Lyrics => S::Lyrics,
        AppAction::Shuffle => S::Shuffle,
        AppAction::ExpandedPlayer => S::ExpandedPlayer,
        AppAction::MiniPlayer => S::MiniPlayer,
        _ => return None,
    })
}
