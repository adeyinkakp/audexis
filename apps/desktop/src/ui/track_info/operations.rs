use super::*;
use audexis_core::database::Database;
use std::sync::atomic::AtomicBool;

const BATCH_SIZE: usize = 25;

fn begin(mut session: State<Session>, progress: String) -> Arc<AtomicBool> {
    let cancel = Arc::new(AtomicBool::new(false));
    let mut state = session.write();
    state.busy = true;
    state.message.clear();
    state.progress = progress;
    state.cancel = Some(cancel.clone());
    cancel
}

fn finish(mut session: State<Session>, message: String) {
    let mut state = session.write();
    state.busy = false;
    state.cancel = None;
    state.progress.clear();
    state.message = message;
    state.revision += 1;
}

pub(super) fn load(
    mut session: State<Session>,
    db: Database,
    runtime: Handle,
    tracks: Vec<(i64, String)>,
) {
    let total = tracks.len();
    let cancel = begin(session, format!("Loading 0 of {total} files…"));
    spawn(async move {
        let mut completed = 0;
        let mut failed = 0;
        let mut errors = Vec::new();
        for chunk in tracks.chunks(BATCH_SIZE) {
            if cancel.load(Ordering::Acquire) {
                break;
            }
            let batch = chunk.to_vec();
            let db = db.clone();
            let stop = cancel.clone();
            let result = runtime
                .spawn(async move {
                    let mut files = Vec::new();
                    let mut failures = Vec::new();
                    for track in batch {
                        if stop.load(Ordering::Acquire) {
                            break;
                        }
                        match metadata_editor::load_library(db.clone(), vec![track]).await {
                            Ok(loaded) => files.extend(loaded),
                            Err(error) => failures.push(error),
                        }
                    }
                    (files, failures)
                })
                .await;
            match result {
                Ok((files, failures)) => {
                    completed += files.len() + failures.len();
                    failed += failures.len();
                    errors.extend(
                        failures
                            .into_iter()
                            .take(10usize.saturating_sub(errors.len())),
                    );
                    let mut state = session.write();
                    state.files.extend(files);
                    state.progress = format!("Loading {completed} of {total} files…");
                }
                Err(error) => {
                    session.write().files.clear();
                    finish(
                        session,
                        format!(
                            "Could not load metadata: {error}. Reopen Track Info to try again."
                        ),
                    );
                    return;
                }
            }
        }
        if cancel.load(Ordering::Acquire) {
            session.write().files.clear();
            finish(
                session,
                "Loading cancelled. No files changed. Reopen Track Info to try again.".into(),
            );
        } else if failed > 0 {
            let loaded = session.peek().files.len();
            finish(
                session,
                format!(
                    "Loaded {loaded} of {total} files; {failed} failed. Edits apply only to loaded files.\n{}",
                    errors.join("\n")
                ),
            );
        } else {
            finish(session, format!("Loaded {total} files."));
        }
    });
}

pub(super) fn save(
    mut session: State<Session>,
    services: Arc<AppServices>,
    runtime: Handle,
    tab: Tab,
) {
    let snapshot = Arc::new(session.peek().clone());
    let indices: Vec<_> = snapshot
        .files
        .iter()
        .enumerate()
        .filter(|(index, file)| {
            snapshot.includes_file(*index, tab)
                && file
                    .changed_keys_matching(|key| tab.includes(key))
                    .next()
                    .is_some()
        })
        .map(|(index, _)| index)
        .collect();
    let total = indices.len();
    let cancel = begin(session, format!("Saving 0 of {total} files…"));
    spawn(async move {
        let mut attempted = 0;
        let mut saved = 0;
        let mut failed = 0;
        let mut errors = Vec::new();
        for chunk in indices.chunks(BATCH_SIZE) {
            if cancel.load(Ordering::Acquire) {
                break;
            }
            let batch = chunk.to_vec();
            let snapshot = snapshot.clone();
            let db = services.db.clone();
            let stop = cancel.clone();
            let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
            let task = runtime.spawn(async move {
                let files = batch
                    .into_iter()
                    .map(|index| snapshot.scoped_file(index, tab))
                    .collect();
                metadata_editor::save_with_progress(db, files, stop, |result| {
                    let _ = sender.send(metadata_editor::SaveResult {
                        attempted: result.attempted,
                        saved: result.saved.clone(),
                        failures: result.failures.clone(),
                    });
                })
                .await;
            });
            let mut changed = Vec::new();
            while let Some(result) = receiver.recv().await {
                attempted += result.attempted;
                saved += result.saved.len();
                failed += result.failures.len();
                errors.extend(
                    result
                        .failures
                        .into_iter()
                        .take(10usize.saturating_sub(errors.len())),
                );
                changed.extend(result.saved.iter().map(|file| file.id));
                let mut state = session.write();
                for file in result.saved {
                    state.accept(file, tab);
                }
                state.progress = format!("Saving {attempted} of {total} files… ({saved} saved)");
            }
            services.library_files.publish_changed(changed);
            services.sync_edits();
            if let Err(error) = task.await {
                failed += 1;
                errors.push(format!(
                    "Save task stopped: {error}. Check files before retrying."
                ));
                break;
            }
        }
        if saved > 0 && tab == Tab::Artwork {
            let cache = services.artwork.clone();
            let _ = runtime.spawn(async move { cache.invalidate().await }).await;
        }
        let outcome = if cancel.load(Ordering::Acquire) {
            "Cancelled"
        } else {
            "Finished"
        };
        finish(
            session,
            format!(
                "{outcome}: processed {attempted} of {total}; saved {saved}; {failed} error(s). Unsaved changes remain in the editor.{}",
                if errors.is_empty() {
                    String::new()
                } else {
                    format!("\n{}", errors.join("\n"))
                }
            ),
        );
    });
}
