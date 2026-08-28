//! Database connection management and query modules.
//!
//! This module provides:
//! - Connection pool creation with retry and exponential backoff
//! - Domain-specific repository modules (`pools`, `predictions`, `referrals`)
//! - Connection pool metrics collection
//!
//! All public items from sub-modules are re-exported at the `crate::db` level
//! so existing callers (`crate::db::get_active_pools`, etc.) continue to work
//! without any changes.

mod pools;
mod predictions;
mod referrals;
pub mod metrics;

// ── Re-export every public item from each repository module ──────────────────
// This keeps the `crate::db::*` call surface identical to the original
// monolithic file — no changes needed in routes, workers, or other callers.

pub use pools::{
    // Types
    CreatorStats,
    OutcomeOdds,
    PoolCreatedEvent,
    PoolDetails,
    PoolRow,
    PoolTemplate,
    PoolWithOdds,
    // Pool queries
    cancel_pool_in_db,
    count_pools_with_filters,
    get_active_pools,
    get_pool_by_id,
    get_pool_outcome_stakes,
    get_pool_with_odds,
    get_pools_with_filters,
    insert_pool_from_event,
    // Business logic
    calculate_odds,
    // Creator incentives
    calculate_creator_incentive,
    get_creator_stats,
    is_creator_reward_eligible,
    pay_creator_incentive,
    record_pool_created_for_creator,
    resolve_pool_in_db,
    // Pool templates
    advance_pool_template,
    create_pool_template,
    get_due_pool_templates,
    list_pool_templates,
};

pub use predictions::{
    // Types
    LeaderboardEntry,
    MarketPredictionRow,
    PredictionHistoryRow,
    PredictionPlacedEvent,
    ProtocolStats,
    UserBettingVolume,
    UserPrediction,
    UserWinnings,
    // Prediction queries
    count_market_predictions,
    get_leaderboard_extended,
    get_market_predictions,
    get_protocol_stats,
    get_user_prediction_history,
    get_user_predictions,
    get_users_by_betting_volume,
    get_users_by_winnings,
    insert_prediction_from_event,
    insert_prediction_from_event_with_pool,
};

pub use referrals::{
    // Types
    ReferralEarningRow,
    ReferralPaidEvent,
    // Referral queries
    get_referral_earnings,
    insert_referral_from_event,
    insert_referrals_bulk,
};

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::time::sleep;
use tracing::{error, info, warn};

use crate::config::Config;

// ── Pool creation error ───────────────────────────────────────────────────────

/// Error returned when the database pool cannot be created after all retries.
#[derive(Debug)]
pub struct PoolCreationError {
    /// The last error encountered during connection attempts.
    pub last_error: sqlx::Error,
    /// Number of attempts made before giving up.
    pub attempts: u32,
}

impl std::fmt::Display for PoolCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to create database pool after {} attempts: {}",
            self.attempts, self.last_error
        )
    }
}

impl std::error::Error for PoolCreationError {}

// ── Transient-error classification ───────────────────────────────────────────

/// Return `true` when `err` is a transient network/timeout error that is safe
/// to retry, or `false` for permanent failures (bad credentials, missing DB).
pub fn is_transient_error(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::PoolTimedOut => true,
        sqlx::Error::PoolClosed => false,
        sqlx::Error::Database(_) => false,
        sqlx::Error::Io(_) | sqlx::Error::Tls(_) => true,
        _ => false,
    }
}

// ── Pool creation with retry ──────────────────────────────────────────────────

/// Create a PostgreSQL connection pool, retrying transient failures with
/// exponential backoff.
///
/// Configuration knobs (`db_connect_max_attempts`, `db_connect_base_delay_ms`,
/// `db_connect_max_delay_ms`) are all validated at startup by
/// [`crate::config::Config::validate`].
pub async fn create_pool(config: &Config) -> Result<PgPool, PoolCreationError> {
    let connect = || async {
        let future = PgPoolOptions::new()
            .max_connections(config.db_max_connections)
            .min_connections(config.db_min_connections)
            .acquire_timeout(Duration::from_secs(config.db_acquire_timeout_secs))
            .connect(&config.database_url);

        match tokio::time::timeout(Duration::from_secs(config.db_connect_timeout_secs), future)
            .await
        {
            Ok(result) => result,
            Err(_) => Err(sqlx::Error::PoolTimedOut),
        }
    };

    retry_pool_connection(
        config.db_connect_max_attempts,
        config.db_connect_base_delay_ms,
        config.db_connect_max_delay_ms,
        connect,
    )
    .await
}

// ── Backoff maths ─────────────────────────────────────────────────────────────

/// Compute the delay in ms before attempt `attempt` (1-based) using
/// truncated binary-exponential backoff.
pub(crate) fn backoff_delay_ms(attempt: u32, base_delay_ms: u64, max_delay_ms: u64) -> u64 {
    let exponent = attempt.saturating_sub(1).min(31);
    let delay = base_delay_ms.saturating_mul(1u64 << exponent);
    delay.min(max_delay_ms)
}

// ── Retry loop ────────────────────────────────────────────────────────────────

async fn retry_pool_connection<Fut, F>(
    max_attempts: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
    mut op: F,
) -> Result<PgPool, PoolCreationError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<PgPool, sqlx::Error>>,
{
    let max_attempts = max_attempts.max(1);
    let mut last_error: Option<sqlx::Error> = None;

    for attempt in 1..=max_attempts {
        match op().await {
            Ok(pool) => {
                if attempt > 1 {
                    info!(
                        attempts = attempt,
                        "database connection established after retries"
                    );
                }
                return Ok(pool);
            }
            Err(err) => {
                let transient = is_transient_error(&err);

                if !transient {
                    error!(
                        attempt,
                        error = %err,
                        "database connection failed with unrecoverable error; aborting"
                    );
                    return Err(PoolCreationError {
                        last_error: err,
                        attempts: attempt,
                    });
                }

                if attempt < max_attempts {
                    let delay_ms = backoff_delay_ms(attempt, base_delay_ms, max_delay_ms);
                    warn!(
                        attempt,
                        max_attempts,
                        delay_ms,
                        error = %err,
                        "database connection failed; retrying"
                    );
                    last_error = Some(err);
                    if delay_ms > 0 {
                        sleep(Duration::from_millis(delay_ms)).await;
                    }
                } else {
                    last_error = Some(err);
                }
            }
        }
    }

    let last_error = last_error.expect("at least one error should exist after retry loop");
    error!(
        attempts = max_attempts,
        error = %last_error,
        "database connection retries exhausted"
    );
    Err(PoolCreationError {
        last_error,
        attempts: max_attempts,
    })
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    };

    #[test]
    fn backoff_delay_is_exponential_and_capped() {
        assert_eq!(backoff_delay_ms(1, 200, 5_000), 200);
        assert_eq!(backoff_delay_ms(2, 200, 5_000), 400);
        assert_eq!(backoff_delay_ms(3, 200, 5_000), 800);
        assert_eq!(backoff_delay_ms(10, 200, 5_000), 5_000);
    }

    #[test]
    fn backoff_delay_with_zero_base_is_zero() {
        assert_eq!(backoff_delay_ms(1, 0, 5_000), 0);
        assert_eq!(backoff_delay_ms(5, 0, 5_000), 0);
    }

    #[test]
    fn backoff_delay_saturates_at_max() {
        assert_eq!(backoff_delay_ms(30, 100, 1_000), 1_000);
        assert_eq!(backoff_delay_ms(64, 1, 500), 500);
    }

    #[test]
    fn config_connect_timeout_is_independent_from_acquire_timeout() {
        let config = crate::config::Config::default_for_test();
        assert!(config.db_connect_timeout_secs > 0, "connect timeout must be > 0");
        assert!(config.db_acquire_timeout_secs > 0, "acquire timeout must be > 0");
    }

    #[test]
    fn is_transient_error_identifies_pool_timeout() {
        assert!(is_transient_error(&sqlx::Error::PoolTimedOut));
        assert!(!is_transient_error(&sqlx::Error::PoolClosed));
    }

    #[test]
    fn pool_creation_error_formats_last_error_and_attempts() {
        let err = PoolCreationError {
            last_error: sqlx::Error::PoolTimedOut,
            attempts: 5,
        };
        let msg = err.to_string();
        assert!(msg.contains("5 attempts"));
    }

    #[tokio::test]
    async fn retry_pool_connection_retries_on_transient_errors() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        let result = retry_pool_connection(3, 0, 0, || async {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Err(sqlx::Error::PoolTimedOut)
        })
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.attempts, 3, "should retry all attempts on transient errors");
    }

    #[tokio::test]
    async fn retry_pool_connection_fails_fast_on_unrecoverable_error() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        let result = retry_pool_connection(5, 0, 0, || async {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Err(sqlx::Error::PoolClosed)
        })
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.attempts, 1, "should fail after only one attempt for PoolClosed");
    }
}
