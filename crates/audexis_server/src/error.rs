use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
#[derive(Debug)]
pub struct Error(pub StatusCode, pub String);
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) type StartupResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
impl Error {
    pub fn bad(message: impl Into<String>) -> Self {
        Self(StatusCode::BAD_REQUEST, message.into())
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let retry = if self.1 == "Three uploads are already pending" {
            "3"
        } else {
            "60"
        };
        let capped = self.0 == StatusCode::SERVICE_UNAVAILABLE
            && self.1.starts_with("Storage daily limit reached");
        let busy = self.0 == StatusCode::TOO_MANY_REQUESTS;
        let mut response = (self.0, Json(serde_json::json!({"error":self.1}))).into_response();
        response
            .headers_mut()
            .insert("cache-control", "no-store".parse().unwrap());
        if busy || self.0 == StatusCode::SERVICE_UNAVAILABLE {
            response
                .headers_mut()
                .insert("retry-after", retry.parse().unwrap());
        }
        if capped {
            response.headers_mut().insert(
                "retry-after",
                crate::storage::cooldown_seconds()
                    .max(60)
                    .to_string()
                    .parse()
                    .unwrap(),
            );
        }
        response
    }
}
impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(%error,"Cloud database operation failed");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database operation failed".into(),
        )
    }
}
impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        Self::bad("Invalid cloud document")
    }
}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        tracing::error!(%error,"Storage I/O failed");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Storage operation failed".into(),
        )
    }
}
