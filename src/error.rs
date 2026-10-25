//! Application error type that renders as a JSON HTTP response.

use actix_web::{
    http::{header, StatusCode},
    HttpResponse, ResponseError,
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("bad request: {0}")]
    BadRequest(String),
    /// Missing, invalid or expired credentials (401).
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    /// Authenticated, but not allowed to do this (403).
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal server error")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// JSON body of every error response.
#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    #[schema(example = "not found")]
    pub error: String,
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        if let AppError::Internal(source) = self {
            log::error!("internal error: {source}");
        }
        let mut response = HttpResponse::build(self.status_code());
        if let AppError::Unauthorized(_) = self {
            response.insert_header((header::WWW_AUTHENTICATE, "Bearer"));
        }
        response.json(ErrorBody {
            error: self.to_string(),
        })
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => AppError::NotFound,
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                AppError::Conflict("resource already exists".into())
            }
            _ => AppError::Internal(Box::new(err)),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
