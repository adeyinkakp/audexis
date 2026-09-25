use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]

pub struct DatabaseMediaFile {
    pub id: i64,
    pub path: String,
    pub file_name: String,
    pub last_validated: i64,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub format: Option<String>,
    pub modified_at: i64,
    pub size: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct DatabaseMediaMetadata {
    pub file_id: i64,
    pub key: String,
    pub value: String,
    pub ord: i64,
}
