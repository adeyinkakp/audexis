mod auth;
mod config;
mod db;
mod error;
mod ingest;
mod router;
mod routes;
mod state;
mod storage;
mod transfer;
mod users;
mod variants;

use error::StartupResult;
pub use state::AppState;
use std::sync::Arc;

#[tokio::main]
async fn main() -> StartupResult<()> {
    let command = users::parse(&std::env::args().skip(1).collect::<Vec<_>>())?;
    if matches!(command, Some(users::Command::Help)) {
        users::help();
        return Ok(());
    }
    config::load_env()?;
    tracing_subscriber::fmt::init();
    if let Some(command) = command {
        let db = db::connect().await?;
        return users::run(&db, command).await;
    }
    let config = config::ServerConfig::from_env()?;
    let bind = config.bind.clone();
    let state = Arc::new(AppState::new(db::connect().await?, config).await?);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    let processing = tokio::spawn(ingest::run(state.clone()));
    let variants = tokio::spawn(variants::run(state.clone()));
    let result = axum::serve(
        listener,
        router::router(state).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await;
    processing.abort();
    variants.abort();
    result?;
    Ok(())
}
