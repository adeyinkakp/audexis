use super::*;

pub fn window_config(
    services: Arc<AppServices>,
    runtime: Handle,
    tracks: Vec<(i64, String)>,
    tab: Tab,
) -> WindowConfig {
    let guard = Arc::new(AtomicU8::new(0));
    let close_guard = guard.clone();
    crate::ui::windows::title_bar::configure(WindowConfig::new_app(EditorApp {
        services,
        runtime,
        tracks,
        tab,
        guard,
    }))
    .with_title("Track Info")
    .with_size(1000., 800.)
    .with_min_size(800., 600.)
    .with_on_close(move |_, _| match close_guard.load(Ordering::Acquire) {
        2 => CloseDecision::KeepOpen,
        1 => {
            let discard = rfd::MessageDialog::new()
                .set_title("Unsaved metadata")
                .set_description("Discard your unsaved changes and close Track Info?")
                .set_buttons(rfd::MessageButtons::YesNo)
                .show();
            if discard == rfd::MessageDialogResult::Yes {
                CloseDecision::Close
            } else {
                CloseDecision::KeepOpen
            }
        }
        _ => CloseDecision::Close,
    })
}

pub(super) struct EditorApp {
    services: Arc<AppServices>,
    runtime: Handle,
    tracks: Vec<(i64, String)>,
    tab: Tab,
    guard: Arc<AtomicU8>,
}
impl App for EditorApp {
    fn render(&self) -> impl IntoElement {
        use_provide_context({
            let services = self.services.clone();
            move || services
        });
        use_provide_context({
            let runtime = self.runtime.clone();
            move || runtime
        });
        crate::ui::theme::use_app_theme(self.services.clone());
        let mut session = use_state(|| Session {
            busy: true,
            ..Session::default()
        });
        use_hook({
            let tracks = self.tracks.clone();
            let db = self.services.db.clone();
            let runtime = self.runtime.clone();
            move || {
                spawn(async move {
                    let result = runtime
                        .spawn(async move { metadata_editor::load_library(db, tracks).await })
                        .await;
                    let mut s = session.write();
                    s.busy = false;
                    match result {
                        Ok(Ok(files)) => s.files = files,
                        other => s.message = format!("Could not read metadata: {other:?}"),
                    }
                });
            }
        });
        use_provide_context(move || session);
        let current = session.read();
        self.guard.store(
            if current.busy {
                2
            } else if current.dirty() {
                1
            } else {
                0
            },
            Ordering::Release,
        );
        drop(current);
        Editor {
            initial_tab: self.tab,
        }
    }
}
