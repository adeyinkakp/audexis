use super::*;

fn track(id: i64, occurrence: Option<i64>) -> UnresolvedTrack {
    UnresolvedTrack {
        id,
        path: format!("/music/{id}.mp3"),
        occurrence,
    }
}

fn queue() -> Queue {
    let (sender, _) = crossbeam_channel::unbounded();
    let mut queue = Queue::new(sender);
    queue.replace_with((1..=3).map(|id| track(id, Some(id - 1))).collect(), 0);
    queue
}

#[test]
fn repeat_off_stops_at_end_and_previous_returns_to_last_track() {
    let mut queue = queue();
    assert!(queue.advance_after_eof());
    assert_eq!(queue.current_path().as_deref(), Some("/music/2.mp3"));
    assert!(queue.advance_after_eof());
    assert!(!queue.advance_after_eof());
    assert_eq!(queue.current_path(), None);
    assert_eq!(queue.previous().as_deref(), Some("/music/3.mp3"));
}

#[test]
fn repeat_track_replays_on_eof_but_manual_next_advances() {
    let mut queue = queue();
    queue.set_repeat_mode(RepeatMode::Track);
    let current = queue.current_queue_id();
    assert!(queue.advance_after_eof());
    assert_eq!(queue.current_queue_id(), current);
    assert_eq!(queue.next().as_deref(), Some("/music/2.mp3"));
    queue.index = 2;
    assert!(queue.next().is_none());
}

#[test]
fn repeat_queue_wraps_in_both_directions() {
    let mut queue = queue();
    queue.set_repeat_mode(RepeatMode::Queue);
    assert_eq!(queue.previous().as_deref(), Some("/music/3.mp3"));
    assert!(queue.advance_after_eof());
    assert_eq!(queue.current_path().as_deref(), Some("/music/1.mp3"));
    queue.index = 2;
    assert_eq!(queue.next().as_deref(), Some("/music/1.mp3"));
}

#[test]
fn empty_queue_is_safe_in_every_repeat_mode() {
    let mut queue = queue();
    queue.clear();
    for mode in [RepeatMode::Off, RepeatMode::Track, RepeatMode::Queue] {
        queue.set_repeat_mode(mode);
        assert!(!queue.advance_after_eof());
        assert!(queue.next().is_none());
        assert!(queue.previous().is_none());
    }
}

#[test]
fn shuffle_round_trip_preserves_duplicate_occurrences_and_current_track() {
    let mut queue = queue();
    queue.append(track(2, Some(3)));
    queue.index = 3;
    let current = queue.current_queue_id();
    let original = queue.original_queue_ids.clone();
    queue.toggle_shuffle();
    assert_eq!(queue.current_queue_id(), current);
    let mut ids = queue.file_ids();
    ids.sort();
    assert_eq!(ids, vec![1, 2, 2, 3]);
    queue.toggle_shuffle();
    assert_eq!(queue.current_queue_id(), current);
    assert_eq!(queue.index, 3);
    let restored: Vec<_> = queue
        .tracks
        .iter()
        .map(|t| t.lock().unwrap().queue_id.clone())
        .collect();
    assert_eq!(restored, original);
}

#[test]
fn insert_next_while_shuffled_survives_restoring_order() {
    let mut queue = queue();
    queue.index = 1;
    queue.toggle_shuffle();
    queue.insert_next(track(4, None));
    assert_eq!(queue.file_ids()[1], 4);
    queue.toggle_shuffle();
    assert_eq!(queue.file_ids(), vec![1, 2, 4, 3]);
    assert_eq!(queue.current_path().as_deref(), Some("/music/2.mp3"));
}

#[test]
fn playlist_removal_preserves_current_duplicate_and_updates_ordinals() {
    let mut queue = queue();
    queue.append(track(2, Some(3)));
    queue.playlist_id = Some(7);
    queue.index = 3;
    let current = queue.current_queue_id();
    assert!(!queue.remove_for_playlist(8, 1));
    assert_eq!(queue.file_ids(), vec![1, 2, 3, 2]);
    assert!(!queue.remove_for_playlist(7, 1));
    assert_eq!(queue.current_queue_id(), current);
    assert_eq!(queue.file_ids(), vec![1, 3, 2]);
    assert_eq!(queue.playlist_ordinals(), vec![Some(0), Some(1), Some(2)]);
}

#[test]
fn removing_all_missing_tracks_leaves_a_safe_empty_queue() {
    let mut queue = queue();
    queue.toggle_shuffle();
    let result = queue.reconcile_library_files(
        &std::collections::HashMap::new(),
        &std::collections::HashSet::from([1, 2, 3]),
    );
    assert!(result.current_removed);
    assert!(result.current_path.is_none());
    queue.toggle_shuffle();
    assert!(queue.file_ids().is_empty());
    assert!(queue.original_queue_ids.is_empty());
    assert!(!queue.advance_after_eof());
}

#[test]
fn replacing_queue_clears_old_playlist_and_modes() {
    let mut queue = queue();
    queue.playlist_id = Some(7);
    queue.set_repeat_mode(RepeatMode::Queue);
    queue.toggle_shuffle();
    queue.replace_with(vec![track(4, None)], 99);
    assert_eq!(queue.current_path().as_deref(), Some("/music/4.mp3"));
    assert_eq!(queue.repeat_mode, RepeatMode::Off);
    assert!(!queue.shuffled);
    assert_eq!(queue.playlist_id, None);
}
