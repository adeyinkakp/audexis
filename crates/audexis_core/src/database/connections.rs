use super::Database;

impl Database {
    pub async fn connection_id(
        &self,
        server_url: &str,
        server_user_id: &str,
    ) -> Result<String, sqlx::Error> {
        sqlx::query_scalar(
            "INSERT INTO connections(id, server_url, server_user_id) VALUES (?1, ?2, ?3)
             ON CONFLICT(server_url, server_user_id) DO UPDATE SET server_url=excluded.server_url
             RETURNING id",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(server_url)
        .bind(server_user_id)
        .fetch_one(&self.pool)
        .await
    }
}
