use std::fmt;

/// Centralized application error type for mapping lower-level errors
/// (like database errors) to high-level variants used by the API.
#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    Conflict(String),
    InvalidInput(String),
    ServiceUnavailable(String),
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::ServiceUnavailable(msg) => write!(f, "service unavailable: {}", msg),

            AppError::NotFound(msg) => write!(f, "not found: {}", msg),
            AppError::Conflict(msg) => write!(f, "conflict: {}", msg),
            AppError::InvalidInput(msg) => write!(f, "invalid input: {}", msg),
            AppError::Internal(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => AppError::NotFound("record not found".to_string()),
            sqlx::Error::Database(db_err) => {
                // Database error may contain SQLSTATE code for Postgres
                if let Some(code) = db_err.code() {
                    match code {
                        // unique_violation
                        std::borrow::Cow::Borrowed("23505") => {
                            AppError::Conflict(db_err.message().to_string())
                        }
                        // foreign_key_violation
                        std::borrow::Cow::Borrowed("23503") => {
                            AppError::Conflict(db_err.message().to_string())
                        }
                        // not_null_violation
                        std::borrow::Cow::Borrowed("23502") => {
                            AppError::InvalidInput(db_err.message().to_string())
                        }
                        _ => AppError::Internal(db_err.message().to_string()),
                    }
                } else {
                    AppError::Internal(db_err.message().to_string())
                }
            }
            other => AppError::Internal(other.to_string()),
        }
    }
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use crate::response::error_codes;
        use crate::response::ApiResponse;
        use axum::http::StatusCode;

        match self {
            AppError::NotFound(msg) => {
                ApiResponse::<()>::error(StatusCode::NOT_FOUND, error_codes::NOT_FOUND, msg)
                    .into_response()
            }
            AppError::Conflict(msg) => {
                ApiResponse::<()>::error(StatusCode::CONFLICT, error_codes::CONFLICT, msg)
                    .into_response()
            }
            AppError::InvalidInput(msg) => {
                ApiResponse::<()>::error(StatusCode::BAD_REQUEST, error_codes::INVALID_INPUT, msg)
                    .into_response()
            }
            AppError::ServiceUnavailable(msg) => ApiResponse::<()>::error(
                StatusCode::SERVICE_UNAVAILABLE,
                error_codes::SERVICE_UNAVAILABLE,
                msg,
            )
            .into_response(),
            AppError::Internal(msg) => ApiResponse::<()>::error(
                StatusCode::INTERNAL_SERVER_ERROR,
                error_codes::INTERNAL_ERROR,
                msg,
            )
            .into_response(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[test]
    fn display_not_found() {
        let err = AppError::NotFound("pool 42".to_string());
        assert_eq!(err.to_string(), "not found: pool 42");
    }

    #[test]
    fn display_conflict() {
        let err = AppError::Conflict("duplicate entry".to_string());
        assert_eq!(err.to_string(), "conflict: duplicate entry");
    }

    #[test]
    fn display_invalid_input() {
        let err = AppError::InvalidInput("bad field".to_string());
        assert_eq!(err.to_string(), "invalid input: bad field");
    }

    #[test]
    fn display_service_unavailable() {
        let err = AppError::ServiceUnavailable("db down".to_string());
        assert_eq!(err.to_string(), "service unavailable: db down");
    }

    #[test]
    fn display_internal() {
        let err = AppError::Internal("unexpected".to_string());
        assert_eq!(err.to_string(), "internal error: unexpected");
    }

    #[test]
    fn not_found_maps_to_404() {
        let resp = AppError::NotFound("x".to_string()).into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn conflict_maps_to_409() {
        let resp = AppError::Conflict("x".to_string()).into_response();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn invalid_input_maps_to_400() {
        let resp = AppError::InvalidInput("x".to_string()).into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn service_unavailable_maps_to_503() {
        let resp = AppError::ServiceUnavailable("x".to_string()).into_response();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn internal_maps_to_500() {
        let resp = AppError::Internal("x".to_string()).into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
