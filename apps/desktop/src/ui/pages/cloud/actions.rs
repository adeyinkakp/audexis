use super::*;

pub(super) fn dispatch(
    services: Arc<AppServices>,
    runtime: tokio::runtime::Handle,
    mut busy: State<bool>,
    mut message: State<Option<String>>,
    mut refresh: State<u64>,
) -> impl FnMut(Action) + Clone {
    move |Action::Retry(id, keep_local)| {
        busy.set(true);
        message.set(None);
        let context = services.cloud_context();
        let services = services.clone();
        let runtime = runtime.clone();
        spawn(async move {
            let result = match context {
                Err(error) => Err(error),
                Ok(context) => runtime
                    .spawn(async move { context.retry(&id, keep_local).await })
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| result.map_err(|error| error.to_string())),
            };
            match result {
                Ok(()) => {
                    services.sync_edits();
                    message.set(Some("Sync change updated.".into()));
                }
                Err(error) => message.set(Some(error)),
            }
            busy.set(false);
            *refresh.write() += 1;
        });
    }
}
