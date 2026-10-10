use crate::utils::errors::DatabaseError;
use sqlx::SqlitePool;

pub async fn get_library_roots(pool: &SqlitePool) -> Result<Vec<String>, DatabaseError> {
    let roots: Vec<String> = sqlx::query_scalar("SELECT path FROM import_roots")
        .fetch_all(pool)
        .await
        .map_err(DatabaseError::Sqlx)?;

    for root in &roots {
        println!("{:?}", root);
    }
    Ok(roots)
}
