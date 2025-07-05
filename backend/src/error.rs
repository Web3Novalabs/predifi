use axum::response::Response;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::json;
use std::fmt;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
#[allow(dead_code)]
pub enum AppError {
    Database(sqlx::Error),
    Configuration(String),
    Internal(String),
    BadRequest(String),
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    ServiceUnavailable(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database(e) => write!(f, "Database error: {}", e),
            AppError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::ServiceUnavailable(msg) => write!(f, "Service unavailable: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!(
            event = "database_error_conversion",
            error = %err,
            error.type = "sqlx::Error",
            "Converting sqlx::Error to AppError"
        );
        AppError::Database(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            AppError::Configuration(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error")
            }
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "Bad request"),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "Not found"),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "Forbidden"),
            AppError::ServiceUnavailable(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "Service unavailable")
            }
        };

        // Log the error with structured context
        match &self {
            AppError::Database(e) => {
                tracing::error!(
                    event = "application_error",
                    error.type = "database",
                    error.message = %e,
                    http.status_code = status.as_u16(),
                    "Database error occurred"
                );
            }
            AppError::Configuration(msg) => {
                tracing::error!(
                    event = "application_error",
                    error.type = "configuration",
                    error.message = msg,
                    http.status_code = status.as_u16(),
                    "Configuration error occurred"
                );
            }
            AppError::Internal(msg) => {
                tracing::error!(
                    event = "application_error",
                    error.type = "internal",
                    error.message = msg,
                    http.status_code = status.as_u16(),
                    "Internal error occurred"
                );
            }
            AppError::BadRequest(msg) => {
                tracing::warn!(
                    event = "application_error",
                    error.type = "bad_request",
                    error.message = msg,
                    http.status_code = status.as_u16(),
                    "Bad request error"
                );
            }
            AppError::NotFound(msg) => {
                tracing::warn!(
                    event = "application_error",
                    error.type = "not_found",
                    error.message = msg,
                    http.status_code = status.as_u16(),
                    "Not found error"
                );
            }
            AppError::Unauthorized(msg) => {
                tracing::warn!(
                    event = "application_error",
                    error.type = "unauthorized",
                    error.message = msg,
                    http.status_code = status.as_u16(),
                    "Unauthorized error"
                );
            }
            AppError::Forbidden(msg) => {
                tracing::warn!(
                    event = "application_error",
                    error.type = "forbidden",
                    error.message = msg,
                    http.status_code = status.as_u16(),
                    "Forbidden error"
                );
            }
            AppError::ServiceUnavailable(msg) => {
                tracing::error!(
                    event = "application_error",
                    error.type = "service_unavailable",
                    error.message = msg,
                    http.status_code = status.as_u16(),
                    "Service unavailable error"
                );
            }
        }

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}

// Helper functions for creating errors with structured logging
impl AppError {
    pub fn database_with_context(e: sqlx::Error, context: &str) -> Self {
        tracing::error!(
            event = "error_creation",
            error.type = "database",
            error.context = context,
            error.source = %e,
            "Creating database error with context"
        );

        AppError::Database(e)
    }

    #[allow(dead_code)]
    pub fn internal_with_context(message: &str, context: &str) -> Self {
        tracing::error!(
            event = "error_creation",
            error.type = "internal",
            error.context = context,
            error.message = message,
            "Creating internal error with context"
        );

        AppError::Internal(format!("{}: {}", context, message))
    }

    #[allow(dead_code)]
    pub fn config_with_context(message: &str, context: &str) -> Self {
        tracing::error!(
            event = "error_creation",
            error.type = "configuration",
            error.context = context,
            error.message = message,
            "Creating configuration error with context"
        );

        AppError::Configuration(format!("{}: {}", context, message))
    }
}
