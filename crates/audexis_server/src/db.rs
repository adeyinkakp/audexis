use crate::error::{Error, Result};
use audexis_core::cloud_sync::protocol::{ChangeValue, Playlist, Track};
use axum::http::StatusCode;
use sqlx::{Postgres, Transaction};

pub async fn lock_account(tx: &mut Transaction<'_, Postgres>, user: &str) -> Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT quota_bytes FROM cloud_accounts WHERE user_id=$1 FOR UPDATE")
            .bind(user)
            .fetch_one(&mut **tx)
            .await?,
    )
}
pub async fn change(
    tx: &mut Transaction<'_, Postgres>,
    user: &str,
    value: &ChangeValue,
) -> Result<()> {
    sqlx::query("INSERT INTO cloud_changes(user_id,value) VALUES ($1,$2)")
        .bind(user)
        .bind(serde_json::to_value(value)?)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub async fn track(tx: &mut Transaction<'_, Postgres>, user: &str, id: &str) -> Result<Track> {
    let value: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT document FROM cloud_tracks WHERE id=$1 AND user_id=$2 AND NOT deleted FOR UPDATE",
    )
    .bind(id)
    .bind(user)
    .fetch_optional(&mut **tx)
    .await?;
    value
        .map(serde_json::from_value)
        .transpose()?
        .ok_or_else(|| Error(StatusCode::NOT_FOUND, "Track not found".into()))
}
pub async fn playlist(
    tx: &mut Transaction<'_, Postgres>,
    user: &str,
    id: &str,
) -> Result<Option<Playlist>> {
    let value:Option<serde_json::Value>=sqlx::query_scalar("SELECT document FROM cloud_playlists WHERE id=$1 AND user_id=$2 AND NOT deleted FOR UPDATE").bind(id).bind(user).fetch_optional(&mut **tx).await?;
    Ok(value.map(serde_json::from_value).transpose()?)
}
pub async fn migrate(pool: &sqlx::PgPool) -> std::result::Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./src/migrations").run(pool).await
}

pub async fn connect() -> std::result::Result<sqlx::PgPool, Box<dyn std::error::Error>> {
    let database = std::env::var("DATABASE_URL")?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&database)
        .await?;
    migrate(&pool).await?;
    Ok(pool)
}
