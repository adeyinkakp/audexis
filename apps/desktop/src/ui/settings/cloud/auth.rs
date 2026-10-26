use super::*;

pub(super) fn login(
    cloud_services: Arc<AppServices>,
    runtime: tokio::runtime::Handle,
    mut logging_in: State<bool>,
    mut login_error: State<Option<String>>,
    mut cancel_login: State<Option<tokio::sync::oneshot::Sender<()>>>,
    mut active_server: State<Option<audexis_core::config::ServerConnection>>,
    mut custom_error: State<Option<String>>,
) {
    logging_in.set(true);
    login_error.set(None);
    let (cancel, cancelled) = tokio::sync::oneshot::channel();
    cancel_login.set(Some(cancel));
    let services = cloud_services.clone();
    let task = runtime.spawn(async move {
        let token = crate::cloud_auth::login(cancelled).await?;
        let saved = services
            .save_server(ServerKind::Cloud, CLOUD_API_URL, &token)
            .await;
        if saved.is_err() {
            let _ = crate::cloud_auth::revoke(&token).await;
        }
        saved
    });
    spawn(async move {
        let result = task.await;
        cancel_login.set(None);
        match result {
            Ok(Ok(connection)) => {
                active_server.set(Some(connection));
                custom_error.set(None);
            }
            Ok(Err(error)) => login_error.set(Some(error.to_string())),
            Err(error) => login_error.set(Some(error.to_string())),
        }
        logging_in.set(false);
    });
}
