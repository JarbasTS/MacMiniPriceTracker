use axum::{
    extract::rejection::{JsonRejection, PathRejection, QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("recurso não encontrado")]
    NotFound,
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound,
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                AppError::BadRequest(format!("valor duplicado ({})", db.constraint().unwrap_or("unique")))
            }
            other => AppError::Internal(other.into()),
        }
    }
}

// Rejeições dos extractors viram 400 em JSON (o padrão do axum é texto puro).
impl From<JsonRejection> for AppError {
    fn from(r: JsonRejection) -> Self {
        AppError::BadRequest(r.body_text())
    }
}
impl From<QueryRejection> for AppError {
    fn from(r: QueryRejection) -> Self {
        AppError::BadRequest(r.body_text())
    }
}
impl From<PathRejection> for AppError {
    fn from(r: PathRejection) -> Self {
        AppError::BadRequest(r.body_text())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Internal(err) => {
                // Detalhe só no log; o cliente recebe mensagem genérica.
                tracing::error!("erro interno: {err:?}");
                (StatusCode::INTERNAL_SERVER_ERROR, "erro interno".to_string())
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
