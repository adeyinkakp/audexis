use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutAction {
    PlayPause,
    NextTrack,
    PreviousTrack,
    SeekForward,
    SeekBackward,
    Shuffle,
    Repeat,
    Favorite,
    Queue,
    Lyrics,
    Search,
    Songs,
    Albums,
    Artists,
    Playlists,
    Favorites,
    NewPlaylist,
    Settings,
    ExpandedPlayer,
    MiniPlayer,
}
impl ShortcutAction {
    pub const ALL: [Self; 20] = [
        Self::PlayPause,
        Self::NextTrack,
        Self::PreviousTrack,
        Self::SeekForward,
        Self::SeekBackward,
        Self::Shuffle,
        Self::Repeat,
        Self::Favorite,
        Self::Queue,
        Self::Lyrics,
        Self::Search,
        Self::Songs,
        Self::Albums,
        Self::Artists,
        Self::Playlists,
        Self::Favorites,
        Self::NewPlaylist,
        Self::Settings,
        Self::ExpandedPlayer,
        Self::MiniPlayer,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::PlayPause => "Play / Pause",
            Self::NextTrack => "Next song",
            Self::PreviousTrack => "Previous song",
            Self::SeekForward => "Seek forward 5 seconds",
            Self::SeekBackward => "Seek backward 5 seconds",
            Self::Shuffle => "Toggle shuffle",
            Self::Repeat => "Cycle repeat mode",
            Self::Favorite => "Toggle favorite",
            Self::Queue => "Show / Hide queue",
            Self::Lyrics => "Show / Hide lyrics",
            Self::Search => "Search library",
            Self::Songs => "Go to songs",
            Self::Albums => "Go to albums",
            Self::Artists => "Go to artists",
            Self::Playlists => "Go to playlists",
            Self::Favorites => "Go to favorites",
            Self::NewPlaylist => "New playlist",
            Self::Settings => "Open settings",
            Self::ExpandedPlayer => "Expand / Collapse player",
            Self::MiniPlayer => "Open mini player",
        }
    }
    pub fn default_binding(self, mac: bool) -> Option<KeyBinding> {
        let (key, shift, alt, primary) = match self {
            Self::PlayPause => ("Space", false, false, false),
            Self::NextTrack => ("ArrowRight", false, false, true),
            Self::PreviousTrack => ("ArrowLeft", false, false, true),
            Self::SeekForward => ("ArrowRight", false, true, false),
            Self::SeekBackward => ("ArrowLeft", false, true, false),
            Self::Shuffle => ("s", true, false, true),
            Self::Repeat => ("r", true, false, true),
            Self::Favorite => ("l", true, false, true),
            Self::Queue => ("j", true, false, true),
            Self::Lyrics => ("k", true, false, true),
            Self::Search => ("f", false, false, true),
            Self::Songs => ("1", false, false, true),
            Self::Albums => ("2", false, false, true),
            Self::Artists => ("3", false, false, true),
            Self::Playlists => ("4", false, false, true),
            Self::Favorites => ("5", false, false, true),
            Self::NewPlaylist => ("n", false, false, true),
            Self::Settings => (",", false, false, true),
            Self::ExpandedPlayer => ("p", true, false, true),
            Self::MiniPlayer => return None,
        };
        Some(KeyBinding {
            key: key.into(),
            control: primary && !mac,
            meta: primary && mac,
            shift,
            alt,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyBinding {
    pub key: String,
    #[serde(default)]
    pub control: bool,
    #[serde(default)]
    pub meta: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
}
impl KeyBinding {
    pub fn label(&self) -> String {
        let mut parts = Vec::new();
        if self.control {
            parts.push("Ctrl".to_string());
        }
        if self.alt {
            parts.push(
                if cfg!(target_os = "macos") {
                    "Option"
                } else {
                    "Alt"
                }
                .to_string(),
            );
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        if self.meta {
            parts.push(
                if cfg!(target_os = "macos") {
                    "Cmd"
                } else {
                    "Super"
                }
                .to_string(),
            );
        }
        parts.push(match self.key.as_str() {
            "ArrowRight" => "→".into(),
            "ArrowLeft" => "←".into(),
            "ArrowUp" => "↑".into(),
            "ArrowDown" => "↓".into(),
            key if key.chars().count() == 1 => key.to_uppercase(),
            key => key.to_string(),
        });
        parts.join(" + ")
    }
    pub fn validate(&self, mac: bool) -> Result<(), String> {
        let primary = if mac { self.meta } else { self.control };
        if matches!(
            self.key.as_str(),
            "Tab" | "Escape" | "Enter" | "Backspace" | "Delete" | "ContextMenu"
        ) {
            return Err("That key is reserved for navigation or editing.".into());
        }
        if !self.control
            && !self.meta
            && !self.alt
            && matches!(
                self.key.as_str(),
                "ArrowUp"
                    | "ArrowDown"
                    | "ArrowLeft"
                    | "ArrowRight"
                    | "Home"
                    | "End"
                    | "PageUp"
                    | "PageDown"
            )
        {
            return Err("Arrow and page keys are reserved for navigation. Add a modifier.".into());
        }
        if primary
            && !self.alt
            && ["a", "c", "v", "x", "z"].contains(&self.key.to_lowercase().as_str())
            || self.control && !self.meta && !self.alt && self.key.eq_ignore_ascii_case("y")
        {
            return Err("That shortcut is reserved for text editing.".into());
        }
        if mac && self.meta && ["q", "w", "h", "m"].contains(&self.key.to_lowercase().as_str())
            || !mac && self.alt && self.key == "F4"
            || self.key == "Delete" && self.control && self.alt
        {
            return Err("That shortcut is reserved by the operating system.".into());
        }
        let named = matches!(
            self.key.as_str(),
            "Space"
                | "ArrowUp"
                | "ArrowDown"
                | "ArrowLeft"
                | "ArrowRight"
                | "Home"
                | "End"
                | "PageUp"
                | "PageDown"
        ) || self
            .key
            .strip_prefix('F')
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=12).contains(&n));
        if !named && (self.key.chars().count() != 1 || self.key.chars().any(char::is_control)) {
            return Err(
                "Choose a letter, number, function key, or a supported navigation key.".into(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardSettings {
    pub bindings: BTreeMap<ShortcutAction, Option<KeyBinding>>,
}
impl KeyboardSettings {
    pub fn binding(&self, action: ShortcutAction, mac: bool) -> Option<KeyBinding> {
        self.bindings
            .get(&action)
            .cloned()
            .unwrap_or_else(|| action.default_binding(mac))
    }
    pub fn assign(
        &mut self,
        action: ShortcutAction,
        mut binding: Option<KeyBinding>,
        mac: bool,
    ) -> Result<(), String> {
        if let Some(binding) = &mut binding {
            if binding.key.chars().count() == 1 {
                binding.key = binding.key.to_lowercase();
            }
        }
        if let Some(binding) = &binding {
            binding.validate(mac)?;
            if let Some(conflict) = ShortcutAction::ALL.into_iter().find(|other| {
                *other != action && self.binding(*other, mac).as_ref() == Some(binding)
            }) {
                return Err(format!(
                    "Already used by {}. Clear or change that shortcut first.",
                    conflict.label()
                ));
            }
        }
        self.bindings.insert(action, binding);
        Ok(())
    }
}
