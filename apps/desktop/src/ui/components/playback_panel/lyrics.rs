use super::lines::LyricLines;
use super::*;

#[derive(PartialEq)]
pub(super) struct LyricsPanel;
impl Component for LyricsPanel {
    fn render(&self) -> impl IntoElement {
        let playback = use_playback();
        let snapshot = playback.snapshot();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let key = (
            snapshot.id,
            snapshot.path.clone().unwrap_or_default(),
            use_library().revision(),
        );
        let source = use_reactive(&key);
        let mut loader = use_hook(move || {
            FutureTask::create(move || {
                let key = source.read().clone();
                let db = services.db.clone();
                let runtime = runtime.clone();
                async move {
                    let requested = key.clone();
                    let task =
                        runtime.spawn(async move { db.lyrics(requested.0, requested.1).await });
                    let _abort = AbortTask(task.abort_handle());
                    let result = task
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                        .map(Arc::new);
                    (key, result)
                }
            })
        });
        use_side_effect_with_deps(&key, move |_| {
            loader.cancel();
            loader.start();
        });
        let colors = use_audexis_colors();
        let state = loader.state();
        let mut retained = use_state(|| None);
        let finished = state.try_as_fulfilled().cloned();
        use_side_effect_with_deps(&finished, move |finished| {
            if let Some((key, Ok(lyrics))) = finished {
                retained.set_if_modified(Some((key.clone(), Ok::<_, String>(lyrics.clone()))));
            }
        });
        let previous = retained.read();
        let matching = |(loaded, _): &&(
            (i64, String, u64),
            Result<Arc<audexis_core::lyrics::Lyrics>, String>,
        )| loaded.0 == key.0 && loaded.1 == key.1;
        let visible = state
            .try_as_fulfilled()
            .filter(matching)
            .or_else(|| previous.as_ref().filter(matching));
        let content = if snapshot.path.is_none() {
            label()
                .text("Play a song to see its lyrics.")
                .into_element()
        } else if let Some((loaded, result)) = visible {
            match result {
                Ok(lyrics) => rect()
                    .key(loaded.1.clone())
                    .expanded()
                    .child(LyricLines {
                        lyrics: lyrics.clone(),
                        position: snapshot.current_position,
                        track_key: loaded.1.clone(),
                    })
                    .into_element(),
                Err(error) => rect()
                    .spacing(12.0)
                    .child(label().text(error.clone()).color(colors.destructive))
                    .child(
                        Button::new()
                            .compact()
                            .flat()
                            .child("Retry")
                            .on_press(move |_| loader.start()),
                    )
                    .into_element(),
            }
        } else {
            label().text("Loading lyrics…").into_element()
        };
        rect().expanded().padding(12.0).child(content)
    }
}
struct AbortTask(tokio::task::AbortHandle);
impl Drop for AbortTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}
