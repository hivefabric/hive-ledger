use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("internal: {0}")]
    Internal(String),
    #[error("postgres: {0}")]
    Pg(#[from] sqlx::Error),
    #[error("migrate: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

pub type LedgerResult<T> = Result<T, LedgerError>;

impl IntoResponse for LedgerError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            LedgerError::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid"),
            LedgerError::Internal(_) | LedgerError::Pg(_) | LedgerError::Migrate(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal")
            }
        };
        let body = Json(json!({
            "error": code,
            "message": self.to_string(),
        }));
        (status, body).into_response()
    }
}
