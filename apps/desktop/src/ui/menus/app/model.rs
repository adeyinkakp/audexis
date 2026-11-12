use super::*;
use crate::ui::keyboard::ShortcutAction as S;

pub(super) fn app(title: &str, action: AppAction) -> Entry {
    Entry::item(title, Action::App(action), true)
}
fn group(title: &str, children: Vec<Entry>) -> Entry {
    Entry {
        title: title.into(),
        action: None,
        enabled: true,
        checked: None,
        children,
    }
}
fn checked(mut entry: Entry, value: bool) -> Entry {
    entry.checked = Some(value);
    entry
}
fn enabled(mut entry: Entry, value: bool) -> Entry {
    entry.enabled = value;
    entry
}
pub(super) fn groups(
    target: &MediaTarget,
    state: &Result<MenuState, String>,
    player: &audexis_audio::PlaybackState,
    available: bool,
    panel: Option<PlaybackPanelView>,
    watcher: bool,
) -> Vec<Entry> {
    let playing = player.id > 0 && available;
    let mut file = vec![
        app("New Playlist…", AppAction::NewPlaylist),
        enabled(app("Import Folder…", AppAction::Import), watcher),
        Entry::separator(),
        group(
            "Library",
            vec![
                app("Manage Music Folders…", AppAction::LibrarySettings),
                enabled(app("Rescan Library", AppAction::Rescan), watcher),
                app("Missing Files", AppAction::Missing),
            ],
        ),
    ];
    if !cfg!(target_os = "macos") || crate::ui::windows::title_bar::windows_title_bar() {
        file.extend([Entry::separator(), app("Settings…", AppAction::Settings)]);
    }
    vec![
        group("File", file),
        group(
            "Edit",
            vec![
                app("Cut", AppAction::Cut),
                app("Copy", AppAction::Copy),
                app("Paste", AppAction::Paste),
                Entry::separator(),
                app("Select All", AppAction::SelectAll),
            ],
        ),
        enabled(
            group("Song", entries(target, available, state, None, true)),
            player.id > 0,
        ),
        group(
            "View",
            vec![
                group(
                    "Go to",
                    vec![
                        app("Search", AppAction::Shortcut(S::Search)),
                        app("Songs", AppAction::Shortcut(S::Songs)),
                        app("Albums", AppAction::Shortcut(S::Albums)),
                        app("Artists", AppAction::Shortcut(S::Artists)),
                        app("Playlists", AppAction::Shortcut(S::Playlists)),
                        app("Favorites", AppAction::Shortcut(S::Favorites)),
                    ],
                ),
                checked(
                    app("Show Queue", AppAction::Queue),
                    panel == Some(PlaybackPanelView::Queue),
                ),
                checked(
                    app("Show Lyrics", AppAction::Lyrics),
                    panel == Some(PlaybackPanelView::Lyrics),
                ),
            ],
        ),
        group(
            "Playback",
            vec![
                app("Expand Player", AppAction::ExpandedPlayer),
                app("Open Mini Player", AppAction::MiniPlayer),
                app("Equalizer…", AppAction::Equalizer),
                Entry::separator(),
                enabled(
                    app(
                        if player.paused { "Play" } else { "Pause" },
                        AppAction::Shortcut(S::PlayPause),
                    ),
                    playing,
                ),
                enabled(app("Previous Track", AppAction::Previous), playing),
                enabled(app("Next Track", AppAction::Next), playing),
                enabled(
                    app("Seek Forward", AppAction::Shortcut(S::SeekForward)),
                    playing,
                ),
                enabled(
                    app("Seek Backward", AppAction::Shortcut(S::SeekBackward)),
                    playing,
                ),
                enabled(
                    app("Toggle Favorite", AppAction::Shortcut(S::Favorite)),
                    playing,
                ),
                enabled(
                    app("Cycle Repeat Mode", AppAction::Shortcut(S::Repeat)),
                    available,
                ),
                Entry::separator(),
                enabled(
                    checked(app("Shuffle", AppAction::Shuffle), player.shuffled),
                    available,
                ),
                group(
                    "Repeat",
                    [
                        ("Off", RepeatMode::Off),
                        ("All", RepeatMode::Queue),
                        ("One", RepeatMode::Track),
                    ]
                    .into_iter()
                    .map(|(label, mode)| {
                        enabled(
                            checked(
                                app(label, AppAction::Repeat(mode)),
                                player.repeat_mode == mode,
                            ),
                            available,
                        )
                    })
                    .collect(),
                ),
            ],
        ),
        group(
            "Help",
            vec![
                app("Report an Issue", AppAction::Issue),
                app("View Logs…", AppAction::Logs),
            ],
        ),
    ]
}
