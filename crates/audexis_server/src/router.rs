use crate::{AppState, auth, routes, transfer};
use axum::{
    Router,
    extract::{DefaultBodyLimit, Request},
    http::{StatusCode, header},
    middleware,
    response::Response,
    routing::{get, post},
};
use std::sync::Arc;

pub(crate) fn router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/session", get(routes::session))
        .route("/usage", get(routes::usage))
        .route("/sync/changes", get(routes::changes))
        .route("/sync/mutations", post(routes::mutate))
        .route("/uploads", post(routes::start_upload))
        .route(
            "/uploads/{id}",
            get(routes::get_upload).delete(routes::cancel_upload),
        )
        .route("/uploads/{id}/complete", post(routes::finish_upload))
        .route("/tracks/{id}/{kind}", get(routes::media))
        .layer(DefaultBodyLimit::max(2_000_000))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::middleware,
        ));
    Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .nest("/api", api)
        .route(
            "/transfer/{token}",
            get(transfer::local_download).put(transfer::local_upload),
        )
        .layer(middleware::from_fn(headers))
        .with_state(state)
}
async fn headers(request: Request, next: middleware::Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    if response.status() == 429 && !response.headers().contains_key(header::RETRY_AFTER) {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, "60".parse().unwrap());
    }
    response
}
