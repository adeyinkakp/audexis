mod actions;
use crate::{
    services::AppServices,
    ui::{components::SmoothScroll, hooks::use_library, theme::use_audexis_colors},
};
use audexis_core::cloud_sync::CloudSyncStatus;
use freya::prelude::*;
use std::sync::Arc;

#[derive(PartialEq)]
pub struct CloudLibrary;
#[derive(Clone, PartialEq)]
struct View {
    connection_id: String,
    jobs: Vec<(String, String, Option<String>)>,
    pending: i64,
}
#[derive(Clone)]
enum Action {
    Retry(String, bool),
}
impl Component for CloudLibrary {
    fn render(&self) -> impl IntoElement {
        let colors = use_audexis_colors();
        let services = use_consume::<Arc<AppServices>>();
        let runtime = use_consume::<tokio::runtime::Handle>();
        let mut refresh = use_state(|| 0u64);
        let mut busy = use_state(|| false);
        let mut message = use_state(|| None::<String>);
        let mut status = use_state(|| services.sync_status.borrow().clone());
        let status_services = services.clone();
        use_future(move || {
            let mut receiver = status_services.sync_status.subscribe();
            async move {
                while receiver.changed().await.is_ok() {
                    status.set(receiver.borrow_and_update().clone());
                }
            }
        });
        let load_services = services.clone();
        let load_runtime = runtime.clone();
        let mut loader = use_hook(move || {
            FutureTask::create(move || {
                let context = load_services.cloud_context();
                let runtime = load_runtime.clone();
                async move {
                    let context = context?;
                    runtime.spawn(async move {
                    let jobs = sqlx::query_as("SELECT operation_id,state,error FROM cloud_outbox WHERE connection_id=? ORDER BY (error IS NOT NULL) DESC,retry_at LIMIT 100")
                        .bind(&context.connection_id).fetch_all(&context.db.pool).await.map_err(|e| e.to_string())?;
                    let pending=sqlx::query_scalar("SELECT count(*) FROM cloud_outbox WHERE connection_id=?").bind(&context.connection_id).fetch_one(&context.db.pool).await.map_err(|e|e.to_string())?;
                    Ok::<_, String>(View { connection_id: context.connection_id.clone(), jobs, pending })
                }).await.map_err(|e| e.to_string())?
                }
            })
        });
        let key = (
            use_library().revision(),
            *refresh.read(),
            use_library().connection(),
            status.read().clone(),
        );
        use_side_effect_with_deps(&key, move |_| {
            loader.cancel();
            loader.start();
        });
        let dispatch = actions::dispatch(services.clone(), runtime.clone(), busy, message, refresh);
        let manual_services = services.clone();
        let usage_services = services.clone();
        let usage_runtime = runtime.clone();
        let mut body = rect()
            .width(Size::fill())
            .padding(24.)
            .spacing(16.)
            .child(
                label()
                    .text("Cloud Sync")
                    .font_size(24.)
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                label()
                    .text(match &*status.read() {
                        CloudSyncStatus::Running => "Syncing…".into(),
                        CloudSyncStatus::Failed(e) => format!("Sync needs attention: {e}"),
                        CloudSyncStatus::Stopped => "Sign in under Settings → Cloud.".into(),
                        CloudSyncStatus::Idle => {
                            if services.user_config.lock().is_ok_and(|c| c.auto_sync) {
                                "Up to date".into()
                            } else {
                                "Automatic uploads paused · Cloud library still updates".into()
                            }
                        }
                    })
                    .font_size(13.)
                    .color(colors.muted_foreground),
            )
            .child(
                label()
                    .text("Your cloud music is available in Songs, Albums, Artists, Favorites, and Playlists. Manage sync and cloud storage here.")
                    .font_size(12.)
                    .color(colors.muted_foreground),
            )
            .child(Button::new().child("Sync now").on_press(move |_| {
                manual_services.sync_now();
                *refresh.write() += 1;
            }))
            .child(
                Button::new()
                    .enabled(!*busy.read())
                    .child("Check storage usage")
                    .on_press(move |_| {
                        let context = usage_services.cloud_context();
                        let rt = usage_runtime.clone();
                        busy.set(true);
                        spawn(async move {
                            let result = match context {
                                Err(e) => Err(e),
                                Ok(c) => rt
                                    .spawn(async move {
                                        c.client()?
                                            .get::<audexis_core::cloud_sync::protocol::Usage>(
                                                "/usage",
                                            )
                                            .await
                                    })
                                    .await
                                    .map_err(|e| e.to_string())
                                    .and_then(|r| r.map_err(|e| e.to_string())),
                            };
                            message.set(Some(match result {
                                Ok(u) => format!(
                                    "{:.2} GB used · {:.2} GB pending · {:.2} GB total",
                                    u.used_bytes as f64 / 1e9,
                                    u.reserved_bytes as f64 / 1e9,
                                    u.quota_bytes as f64 / 1e9
                                ),
                                Err(e) => e,
                            }));
                            busy.set(false);
                        });
                    }),
            )
            .maybe_child(
                message
                    .read()
                    .clone()
                    .map(|m| label().text(m).font_size(13.)),
            );
        let state = loader.state();
        let mut retained = use_state(|| None::<Result<View, String>>);
        let result = state.try_as_fulfilled().cloned();
        use_side_effect_with_deps(&result, move |result| {
            if let Some(Ok(view)) = result {
                retained.set_if_modified(Some(Ok(view.clone())));
            }
        });
        let previous = retained.read();
        let account = use_library().connection();
        let scoped = |result: &&Result<View, String>| {
            result.as_ref().map_or(true, |view| {
                account.as_deref() == Some(view.connection_id.as_str())
            })
        };
        match state
            .try_as_fulfilled()
            .filter(scoped)
            .or_else(|| previous.as_ref().filter(scoped))
        {
            None => body = body.child(label().text("Loading sync status…")),
            Some(Err(e)) => body = body.child(label().text(e.clone()).color(colors.destructive)),
            Some(Ok(view)) => {
                body = body.child(
                    label()
                        .text(format!("{} queued operations", view.pending))
                        .font_size(12.),
                );
                for (id, state, error) in &view.jobs {
                    if let Some(error) = error {
                        let id1 = id.clone();
                        let id2 = id.clone();
                        let mut retry = dispatch.clone();
                        let mut discard = dispatch.clone();
                        body = body.child(
                            rect()
                                .spacing(6.)
                                .child(
                                    label()
                                        .text(format!("{state}: {error}"))
                                        .font_size(12.)
                                        .color(colors.destructive),
                                )
                                .child(
                                    Button::new()
                                        .enabled(!*busy.read())
                                        .child(if state == "conflict" {
                                            "Use my change"
                                        } else {
                                            "Retry"
                                        })
                                        .on_press(move |_| retry(Action::Retry(id1.clone(), true))),
                                )
                                .maybe_child((state == "conflict").then(|| {
                                    Button::new()
                                        .enabled(!*busy.read())
                                        .child("Use cloud version")
                                        .on_press(move |_| {
                                            discard(Action::Retry(id2.clone(), false))
                                        })
                                })),
                        );
                    }
                }
            }
        }
        SmoothScroll::new().child(body.child(rect().height(Size::px(80.))))
    }
}
