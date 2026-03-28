//! # PrediFi Backend Error Handling
//!
//! Provides a unified [`AppError`] enum for all API and database errors,
//! enabling consistent error propagation and HTTP response mapping across
//! the backend service.

use thiserror::Error;

/// Top-level application error.
///
/// All fallible operations in the backend should return `Result<T, AppError>`.
/// Variants are grouped by origin:
/// - [`AppError::Api`]  — request validation / business-logic errors (4xx)
/// - [`AppError::Database`] — storage layer failures (5xx)
#[derive(Debug, Error)]
pub enum AppError {
    // ── API / request errors (4xx) ─────────────────────────────────────────
    /// A required field was missing or a value failed validation.
    #[error("validation error: {0}")]
    Validation(String),

    /// The requested resource does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// The caller is not authorised to perform this action.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    // ── Database / storage errors (5xx) ────────────────────────────────────
    /// A database query failed.
    #[error("database error: {0}")]
    Database(String),

    /// A database connection could not be established.
    #[error("database connection error: {0}")]
    DatabaseConnection(String),
}

impl AppError {
    /// Returns the HTTP status code that best represents this error.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::Validation(_) => 400,
            Self::Unauthorized(_) => 401,
            Self::NotFound(_) => 404,
            Self::Database(_) | Self::DatabaseConnection(_) => 500,
        }
    }

    /// Returns `true` for errors caused by the caller (4xx).
    pub fn is_client_error(&self) -> bool {
        self.status_code() < 500
    }
}

fn main() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_is_client_error() {
        let err = AppError::Validation("amount must be positive".into());
        assert_eq!(err.status_code(), 400);
        assert!(err.is_client_error());
        assert_eq!(err.to_string(), "validation error: amount must be positive");
    }

    #[test]
    fn not_found_error_is_client_error() {
        let err = AppError::NotFound("pool 42".into());
        assert_eq!(err.status_code(), 404);
        assert!(err.is_client_error());
    }

    #[test]
    fn unauthorized_error_is_client_error() {
        let err = AppError::Unauthorized("missing token".into());
        assert_eq!(err.status_code(), 401);
        assert!(err.is_client_error());
    }

    #[test]
    fn database_error_is_server_error() {
        let err = AppError::Database("query timeout".into());
        assert_eq!(err.status_code(), 500);
        assert!(!err.is_client_error());
        assert_eq!(err.to_string(), "database error: query timeout");
    }

    #[test]
    fn database_connection_error_is_server_error() {
        let err = AppError::DatabaseConnection("refused".into());
        assert_eq!(err.status_code(), 500);
        assert!(!err.is_client_error());
    }
}
