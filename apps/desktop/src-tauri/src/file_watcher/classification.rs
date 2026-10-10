use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use notify::{
    event::{ModifyKind, RenameMode},
    EventKind,
};
use notify_debouncer_full::DebouncedEvent;

use super::is_audio_file;

#[derive(Default, Debug, PartialEq, Eq)]
pub(super) struct ClassifiedEvents {
    pub(super) changed: Vec<PathBuf>,
    pub(super) removed: Vec<PathBuf>,
    pub(super) renamed: Vec<(PathBuf, PathBuf)>,
}

pub(super) fn classify_events(events: &[DebouncedEvent]) -> ClassifiedEvents {
    let mut result = ClassifiedEvents::default();
    let mut pending_renames = HashMap::<usize, PathBuf>::new();
    let mut pending_destinations = HashMap::<usize, PathBuf>::new();

    for event in events {
        if let EventKind::Modify(ModifyKind::Name(mode)) = event.kind {
            match mode {
                RenameMode::Both => {
                    if let (Some(old_path), Some(new_path)) =
                        (event.paths.first(), event.paths.get(1))
                    {
                        result
                            .renamed
                            .push((old_path.to_path_buf(), new_path.to_path_buf()));
                    } else if let Some(path) = event.paths.first() {
                        if path.exists() {
                            result.changed.push(path.to_path_buf());
                        } else {
                            result.removed.push(path.to_path_buf());
                        }
                    }
                }
                RenameMode::From => {
                    if let Some(path) = event.paths.first() {
                        if let Some(tracker) = event.attrs.tracker() {
                            if let Some(new_path) = pending_destinations.remove(&tracker) {
                                result.renamed.push((path.to_path_buf(), new_path));
                            } else {
                                pending_renames.insert(tracker, path.to_path_buf());
                            }
                        } else {
                            result.removed.push(path.to_path_buf());
                        }
                    }
                }
                RenameMode::To => {
                    if let Some(new_path) = event.paths.first() {
                        if let Some(tracker) = event.attrs.tracker() {
                            if let Some(old_path) = pending_renames.remove(&tracker) {
                                result.renamed.push((old_path, new_path.to_path_buf()));
                            } else {
                                pending_destinations.insert(tracker, new_path.to_path_buf());
                            }
                        } else {
                            result.changed.push(new_path.to_path_buf());
                        }
                    }
                }
                RenameMode::Any | RenameMode::Other => {
                    if event.paths.len() >= 2 {
                        result.renamed.push((
                            event.paths.first().unwrap().to_path_buf(),
                            event.paths.last().unwrap().to_path_buf(),
                        ));
                    } else if let Some(path) = event.paths.first() {
                        if path.exists() {
                            result.changed.push(path.to_path_buf());
                        } else {
                            result.removed.push(path.to_path_buf());
                        }
                    }
                }
            }
            continue;
        }

        for path in &event.paths {
            match event.kind {
                EventKind::Remove(_) => result.removed.push(path.to_path_buf()),
                EventKind::Create(_) => result.changed.push(path.to_path_buf()),
                EventKind::Modify(_) if is_audio_file(path) => {
                    result.changed.push(path.to_path_buf())
                }
                _ => {}
            }
        }
    }

    result.removed.extend(pending_renames.into_values());
    result.changed.extend(pending_destinations.into_values());
    result.changed = deduplicate_paths(result.changed);
    result.removed = deduplicate_paths(result.removed);
    result.renamed.sort();
    result.renamed.dedup();
    result
}

fn deduplicate_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

#[cfg(test)]
mod rename_event_tests {
    use super::*;
    use notify::Event;

    fn event(kind: EventKind, paths: &[&str], tracker: Option<usize>) -> DebouncedEvent {
        let mut event = Event::new(kind);
        for path in paths {
            event = event.add_path(PathBuf::from(path));
        }
        if let Some(tracker) = tracker {
            event = event.set_tracker(tracker);
        }
        event.into()
    }

    #[test]
    fn classifies_combined_rename() {
        let events = vec![event(
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
            &["/music/old.mp3", "/music/new.mp3"],
            None,
        )];

        let classified = classify_events(&events);
        assert_eq!(
            classified.renamed,
            vec![(
                PathBuf::from("/music/old.mp3"),
                PathBuf::from("/music/new.mp3")
            )]
        );
        assert!(classified.changed.is_empty());
        assert!(classified.removed.is_empty());
    }

    #[test]
    fn pairs_split_rename_by_tracker() {
        let events = vec![
            event(
                EventKind::Modify(ModifyKind::Name(RenameMode::From)),
                &["/music/old.mp3"],
                Some(42),
            ),
            event(
                EventKind::Modify(ModifyKind::Name(RenameMode::To)),
                &["/music/new.mp3"],
                Some(42),
            ),
        ];

        let classified = classify_events(&events);
        assert_eq!(
            classified.renamed,
            vec![(
                PathBuf::from("/music/old.mp3"),
                PathBuf::from("/music/new.mp3")
            )]
        );
        assert!(classified.changed.is_empty());
        assert!(classified.removed.is_empty());
    }

    #[test]
    fn pairs_out_of_order_split_rename_by_tracker() {
        let events = vec![
            event(
                EventKind::Modify(ModifyKind::Name(RenameMode::To)),
                &["/music/new.mp3"],
                Some(42),
            ),
            event(
                EventKind::Modify(ModifyKind::Name(RenameMode::From)),
                &["/music/old.mp3"],
                Some(42),
            ),
        ];

        let classified = classify_events(&events);
        assert_eq!(
            classified.renamed,
            vec![(
                PathBuf::from("/music/old.mp3"),
                PathBuf::from("/music/new.mp3")
            )]
        );
        assert!(classified.changed.is_empty());
        assert!(classified.removed.is_empty());
    }

    #[test]
    fn unmatched_rename_halves_fall_back_to_remove_and_add() {
        let events = vec![
            event(
                EventKind::Modify(ModifyKind::Name(RenameMode::From)),
                &["/music/gone.mp3"],
                Some(1),
            ),
            event(
                EventKind::Modify(ModifyKind::Name(RenameMode::To)),
                &["/music/arrived.mp3"],
                Some(2),
            ),
        ];

        let classified = classify_events(&events);
        assert_eq!(classified.removed, vec![PathBuf::from("/music/gone.mp3")]);
        assert_eq!(
            classified.changed,
            vec![PathBuf::from("/music/arrived.mp3")]
        );
        assert!(classified.renamed.is_empty());
    }

    #[test]
    fn generic_two_path_rename_is_not_dropped() {
        let events = vec![event(
            EventKind::Modify(ModifyKind::Name(RenameMode::Any)),
            &["/music/old", "/music/new"],
            None,
        )];

        assert_eq!(
            classify_events(&events).renamed,
            vec![(PathBuf::from("/music/old"), PathBuf::from("/music/new"))]
        );
    }
}
