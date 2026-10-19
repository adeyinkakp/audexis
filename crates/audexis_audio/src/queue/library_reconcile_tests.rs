use super::*;
use std::collections::{HashMap, HashSet};

fn queue() -> Queue {
    let (sender, _receiver) = crossbeam_channel::unbounded();
    let mut queue = Queue::new(sender);
    for id in 1..=3 {
        queue.append(UnresolvedTrack {
            id,
            path: format!("/music/{id}.mp3"),
            occurrence: None,
        });
    }
    queue
}

#[test]
fn updates_paths_without_changing_current_track() {
    let mut queue = queue();
    queue.index = 1;
    let outcome = queue.reconcile_library_files(
        &HashMap::from([(2, "/renamed/2.mp3".to_string())]),
        &HashSet::new(),
    );

    assert!(!outcome.current_removed);
    assert!(outcome.current_path_changed);
    assert_eq!(outcome.current_path.as_deref(), Some("/renamed/2.mp3"));
    assert_eq!(queue.file_ids(), vec![1, 2, 3]);
}

#[test]
fn removing_current_track_advances_to_next_available_track() {
    let mut queue = queue();
    queue.index = 1;
    let outcome = queue.reconcile_library_files(&HashMap::new(), &HashSet::from([1, 2]));

    assert!(outcome.current_removed);
    assert_eq!(outcome.current_path.as_deref(), Some("/music/3.mp3"));
    assert_eq!(queue.file_ids(), vec![3]);
    assert_eq!(queue.index, 0);
}
