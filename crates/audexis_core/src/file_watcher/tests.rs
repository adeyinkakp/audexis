use super::*;
use crate::tag_manager::{
    tag_backend::{DefaultBackend, TagBackend},
    utils::{Changes, FrameKey, SerializableTagValue, TagChange},
};
use std::{collections::HashMap, path::PathBuf};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("audexis-watcher-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn fixture(path: &Path) {
    std::fs::write(path, b"ID3\x03\0\0\0\0\0\0").unwrap();
}
async fn context(dir: &Directory) -> WatcherContext {
    let db = Database::init(&dir.0.join("library.db")).await.unwrap();
    let files = LibraryFiles::new(
        db.pool.clone(),
        tokio::runtime::Handle::current(),
        |_, _| {},
    );
    let (status, _) = watch::channel(WatcherStatus::default());
    WatcherContext {
        db,
        files,
        library_scan_lock: Arc::default(),
        pending_worker_running: Arc::default(),
        status,
    }
}
fn event(kind: notify::EventKind, paths: &[&Path]) -> DebouncedEvent {
    let mut event = notify::Event::new(kind);
    for path in paths {
        event = event.add_path(path.to_path_buf());
    }
    DebouncedEvent::new(event, std::time::Instant::now())
}
#[tokio::test]
async fn scans_scope_deletions_and_atomic_replacements_preserve_identity() {
    let dir = Directory::new();
    let context = context(&dir).await;
    let music = dir.0.join("music");
    std::fs::create_dir(&music).unwrap();
    sqlx::query("INSERT INTO import_roots(path) VALUES (?)")
        .bind(music.to_str().unwrap())
        .execute(&context.db.pool)
        .await
        .unwrap();
    let mut changes = context.files.subscribe();
    let song = music.join("song.MP3");
    fixture(&song);
    sqlx::query("INSERT INTO files(path,file_name,size) VALUES (?, 'outside.mp3',0)")
        .bind(dir.0.join("outside.mp3").to_str().unwrap())
        .execute(&context.db.pool)
        .await
        .unwrap();
    FileWatcher::scan_folders_for_context(&context)
        .await
        .unwrap();
    let (id, status): (i64, String) = sqlx::query_as("SELECT id,status FROM files WHERE path=?")
        .bind(song.to_str().unwrap())
        .fetch_one(&context.db.pool)
        .await
        .unwrap();
    assert_eq!(status, "ok");
    let outside_missing: Option<i64> =
        sqlx::query_scalar("SELECT missing_since FROM files WHERE file_name='outside.mp3'")
            .fetch_one(&context.db.pool)
            .await
            .unwrap();
    assert!(outside_missing.is_none());
    sqlx::query("INSERT INTO playlists(id,name) VALUES (1,'Saved')")
        .execute(&context.db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO playlist_tracks(playlist_id,file_id,ord) VALUES (1,?,0)")
        .bind(id)
        .execute(&context.db.pool)
        .await
        .unwrap();
    let temporary = music.join(".song.tmp");
    fixture(&temporary);
    std::fs::rename(&temporary, &song).unwrap();
    FileWatcher::handle_events(
        &context,
        &[event(
            notify::EventKind::Modify(notify::event::ModifyKind::Name(
                notify::event::RenameMode::Both,
            )),
            &[&temporary, &song],
        )],
    )
    .await
    .unwrap();
    let linked: i64 = sqlx::query_scalar("SELECT file_id FROM playlist_tracks WHERE playlist_id=1")
        .fetch_one(&context.db.pool)
        .await
        .unwrap();
    assert_eq!(linked, id);
    let path = song.clone();
    let errors = tokio::task::spawn_blocking(move || {
        DefaultBackend::new().write_changes(&Changes {
            paths: vec![path.to_string_lossy().into_owned()],
            tags: HashMap::from([(
                FrameKey::Title,
                TagChange::Replace(vec![SerializableTagValue::Text("Edited title".into())]),
            )]),
        })
    })
    .await
    .unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    FileWatcher::handle_events(
        &context,
        &[event(
            notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Any,
            )),
            &[&song],
        )],
    )
    .await
    .unwrap();
    let title: String =
        sqlx::query_scalar("SELECT value FROM metadata_texts WHERE file_id=? AND key='title'")
            .bind(id)
            .fetch_one(&context.db.pool)
            .await
            .unwrap();
    assert_eq!(title, "Edited title");
    let mut notified = false;
    while let Ok(change) = changes.try_recv() {
        notified |= change.file_ids.contains(&id);
    }
    assert!(
        notified,
        "UI library subscribers receive indexed file changes"
    );

    let renamed = music.join("renamed.mp3");
    std::fs::rename(&song, &renamed).unwrap();
    FileWatcher::handle_events(
        &context,
        &[event(
            notify::EventKind::Modify(notify::event::ModifyKind::Name(
                notify::event::RenameMode::Both,
            )),
            &[&song, &renamed],
        )],
    )
    .await
    .unwrap();
    let current: String = sqlx::query_scalar("SELECT path FROM files WHERE id=?")
        .bind(id)
        .fetch_one(&context.db.pool)
        .await
        .unwrap();
    assert_eq!(Path::new(&current), renamed);
    std::fs::remove_file(&renamed).unwrap();
    FileWatcher::scan_folders_for_context(&context)
        .await
        .unwrap();
    let missing: Option<i64> = sqlx::query_scalar("SELECT missing_since FROM files WHERE id=?")
        .bind(id)
        .fetch_one(&context.db.pool)
        .await
        .unwrap();
    assert!(missing.is_some());
    std::fs::remove_dir(&music).unwrap();
    assert!(
        FileWatcher::scan_folders_for_context(&context)
            .await
            .is_err()
    );
    context.db.pool.close().await;
}

#[tokio::test]
#[ignore = "Requires OS filesystem event delivery; run explicitly outside the sandbox"]
async fn native_watcher_imports_edits_and_removals_publish_to_library_subscribers() {
    let dir = Directory::new();
    let context = context(&dir).await;
    let music = dir.0.join("music");
    std::fs::create_dir(&music).unwrap();
    let mut changes = context.files.subscribe();
    sqlx::query("INSERT INTO import_roots(path) VALUES (?)")
        .bind(
            std::fs::canonicalize(&music)
                .unwrap()
                .to_string_lossy()
                .as_ref(),
        )
        .execute(&context.db.pool)
        .await
        .unwrap();
    let watcher = FileWatcher::start(context.db.clone(), context.files.clone())
        .await
        .unwrap();
    let song = music.join("song.mp3");
    tokio::time::timeout(Duration::from_secs(15), async {
        let mut status = watcher.subscribe();
        loop {
            let current = status.borrow_and_update().clone();
            assert!(current.error.is_none(), "{:?}", current);
            if !current.busy && current.message.starts_with("Updated") {
                break;
            }
            status.changed().await.unwrap();
        }
        fixture(&song);
        loop {
            changes.recv().await.unwrap();
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM files WHERE status='ok'")
                .fetch_one(&context.db.pool)
                .await
                .unwrap();
            if count == 1 {
                break;
            }
        }
        let path = song.clone();
        tokio::task::spawn_blocking(move || {
            let errors = DefaultBackend::new().write_changes(&Changes {
                paths: vec![path.to_string_lossy().into_owned()],
                tags: HashMap::from([(
                    FrameKey::Title,
                    TagChange::Replace(vec![SerializableTagValue::Text("Watcher title".into())]),
                )]),
            });
            assert!(errors.is_empty(), "{errors:?}");
        })
        .await
        .unwrap();
        loop {
            changes.recv().await.unwrap();
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM metadata_texts WHERE value='Watcher title'",
            )
            .fetch_one(&context.db.pool)
            .await
            .unwrap();
            if count > 0 {
                break;
            }
        }
        std::fs::remove_file(&song).unwrap();
        loop {
            changes.recv().await.unwrap();
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM files WHERE missing_since IS NOT NULL")
                    .fetch_one(&context.db.pool)
                    .await
                    .unwrap();
            if count == 1 {
                break;
            }
        }
    })
    .await
    .expect("native file events did not update the library");
    drop(watcher);
    context.db.pool.close().await;
}
