//! # PrediFi Backend
//!
//! Provides unified error handling and core traits for the PrediFi backend service.

use thiserror::Error;

// ── Error types ───────────────────────────────────────────────────────────────

/// Top-level application error.
///
/// All fallible operations should return `Result<T, AppError>`.
/// Variants map directly to HTTP status codes via [`AppError::status_code`].
#[derive(Debug, Error)]
pub enum AppError {
    /// A required field was missing or a value failed validation (400).
    #[error("validation error: {0}")]
    Validation(String),

    /// The caller is not authorised to perform this action (401).
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// The requested resource does not exist (404).
    #[error("not found: {0}")]
    NotFound(String),

    /// A database query failed (500).
    #[error("database error: {0}")]
    Database(String),

    /// A database connection could not be established (500).
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

// ── Pool helper ───────────────────────────────────────────────────────────────

/// Trait for fetching pool data — abstracted so it can be mocked in tests.
#[cfg_attr(test, mockall::automock)]
pub trait PoolRepository {
    /// Returns the total stake for a pool, or an error if not found.
    fn get_total_stake(&self, pool_id: u64) -> Result<u64, AppError>;
}

/// Returns `true` when the pool's total stake meets or exceeds `min_stake`.
///
/// # Errors
/// Propagates any [`AppError`] returned by the repository.
pub fn pool_has_min_stake(
    repo: &dyn PoolRepository,
    pool_id: u64,
    min_stake: u64,
) -> Result<bool, AppError> {
    let stake = repo.get_total_stake(pool_id)?;
    Ok(stake >= min_stake)
}

fn main() {}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::eq;

    #[test]
    fn returns_true_when_stake_meets_minimum() {
        let mut mock = MockPoolRepository::new();
        mock.expect_get_total_stake()
            .with(eq(1u64))
            .returning(|_| Ok(500));

        assert!(pool_has_min_stake(&mock, 1, 500).unwrap());
    }

    #[test]
    fn returns_false_when_stake_below_minimum() {
        let mut mock = MockPoolRepository::new();
        mock.expect_get_total_stake()
            .with(eq(2u64))
            .returning(|_| Ok(99));

        assert!(!pool_has_min_stake(&mock, 2, 100).unwrap());
    }

    #[test]
    fn propagates_not_found_error() {
        let mut mock = MockPoolRepository::new();
        mock.expect_get_total_stake()
            .returning(|id| Err(AppError::NotFound(format!("pool {id}"))));

        let err = pool_has_min_stake(&mock, 99, 1).unwrap_err();
        assert_eq!(err.status_code(), 404);
        assert!(err.is_client_error());
    }

    #[test]
    fn app_error_display_and_status_codes() {
        let cases: &[(AppError, u16, &str)] = &[
            (AppError::Validation("bad input".into()), 400, "validation error: bad input"),
            (AppError::Unauthorized("no token".into()), 401, "unauthorized: no token"),
            (AppError::NotFound("pool 1".into()), 404, "not found: pool 1"),
            (AppError::Database("timeout".into()), 500, "database error: timeout"),
            (AppError::DatabaseConnection("refused".into()), 500, "database connection error: refused"),
        ];
        for (err, code, msg) in cases {
            assert_eq!(err.status_code(), *code);
            assert_eq!(err.to_string(), *msg);
        }
    }
}
