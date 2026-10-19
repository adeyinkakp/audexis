use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Action {
    Play,
    Info,
    Edit,
    Next,
    Last,
    RemoveQueue,
    Copy,
    Heart(bool),
    AddPlaylist(i64),
    ShowAlbum,
    RemovePlaylist,
    App(app::AppAction),
}
#[derive(Clone, PartialEq)]
pub(super) struct Entry {
    pub(super) title: String,
    pub(super) action: Option<Action>,
    pub(super) enabled: bool,
    pub(super) checked: Option<bool>,
    pub(super) children: Vec<Entry>,
}
impl Entry {
    pub(super) fn item(title: impl Into<String>, action: Action, enabled: bool) -> Self {
        Self {
            title: title.into(),
            action: Some(action),
            enabled,
            checked: None,
            children: vec![],
        }
    }
    pub(super) fn separator() -> Self {
        Self {
            title: String::new(),
            action: None,
            enabled: false,
            checked: None,
            children: vec![],
        }
    }
    pub(super) fn message(title: &str) -> Self {
        Self {
            title: title.into(),
            ..Self::separator()
        }
    }
}
pub(super) fn tracks(target: &MediaTarget) -> Vec<(i64, String)> {
    let source = match target {
        MediaTarget::Selection { tracks } => tracks
            .iter()
            .map(|t| (t.track.id, t.track.path.clone()))
            .collect(),
        MediaTarget::Track { track, .. } => vec![(track.id, track.path.clone())],
        MediaTarget::Album { tracks } => tracks.iter().map(|t| (t.id, t.path.clone())).collect(),
    };
    let mut result = Vec::new();
    for (id, path) in source {
        if !result.iter().any(|(previous, _)| *previous == id) {
            result.push((id, path));
        }
    }
    result
}
pub(super) fn entries(
    target: &MediaTarget,
    available: bool,
    state: &Result<MenuState, String>,
    playlist: Option<PlaylistOccurrence>,
    navigation: bool,
) -> Vec<Entry> {
    let ids = tracks(target);
    let indexed = !ids.is_empty() && ids.iter().all(|(id, _)| *id > 0);
    let queue = available && target.enqueue_command(false).is_some();
    let mut heart = Entry::item(
        "Heart",
        Action::Heart(!state.as_ref().is_ok_and(|s| s.all_loved)),
        indexed && state.is_ok(),
    );
    heart.checked = state.as_ref().ok().map(|s| s.all_loved);
    let children = match state {
        Ok(s) if !s.playlists.is_empty() => s
            .playlists
            .iter()
            .map(|(id, name)| {
                Entry::item(
                    name,
                    Action::AddPlaylist(*id),
                    indexed && playlist.is_none_or(|p| p.playlist != *id),
                )
            })
            .collect(),
        Ok(_) => vec![Entry::message("No playlists available")],
        Err(_) => vec![Entry::message("Could not load playlists")],
    };
    let mut items = vec![
        Entry::item(
            "Play",
            Action::Play,
            available && target.play_command().is_some(),
        ),
        Entry::item("Play Next", Action::Next, queue),
        Entry::item("Play Last", Action::Last, queue),
        Entry::separator(),
        Entry::item("Get Info", Action::Info, !target.paths().is_empty()),
        Entry::item(
            "Edit Metadata Fields…",
            Action::Edit,
            !target.paths().is_empty(),
        ),
        heart,
        Entry {
            title: "Add to Playlist".into(),
            action: None,
            enabled: indexed,
            checked: None,
            children,
        },
        Entry::separator(),
        Entry::item(
            "Show Album in Library",
            Action::ShowAlbum,
            indexed && ids.len() == 1 && navigation,
        ),
        Entry::item(
            if ids.len() > 1 {
                "Copy file paths"
            } else {
                "Copy file path"
            },
            Action::Copy,
            !target.paths().is_empty(),
        ),
    ];
    if playlist.is_some() {
        items.push(Entry::separator());
        items.push(Entry::item(
            "Remove from Playlist",
            Action::RemovePlaylist,
            true,
        ));
    }
    if matches!(target, MediaTarget::Selection { tracks } if tracks.iter().all(|t| t.queue_id.is_some()))
        || matches!(
            target,
            MediaTarget::Track {
                queue_id: Some(_),
                ..
            }
        )
    {
        items.push(Entry::separator());
        items.push(Entry::item(
            "Remove from Queue",
            Action::RemoveQueue,
            available,
        ));
    }
    items
}
