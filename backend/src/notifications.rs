//! In-app notification system.
//!
//! Alerts users when: their pool is about to end, a pool they predicted on
//! is resolved, their claim window is about to expire, and when new pools
//! matching their interests are created.
//!
//! Delivery is sweep-based ([`run_notification_sweep`]) rather than wired
//! into every individual event-ingestion path: it's simpler, and the unique
//! `(user_address, pool_id, notif_type)` index on `notifications` makes every
//! insert idempotent, so the sweep can run on a fixed interval (see
//! `server.rs`) without ever double-notifying a user.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use tracing::{info, instrument, warn};

/// How far ahead of `end_time` a pool counts as "ending soon".
const POOL_ENDING_SOON_WINDOW: &str = "1 hour";
/// How far ahead of claim-window expiry a user is warned.
const CLAIM_EXPIRING_WINDOW: &str = "1 day";
/// How far back to scan for newly created pools when matching interests.
const NEW_POOL_LOOKBACK: &str = "1 day";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifType {
    PoolEndingSoon,
    PoolResolved,
    ClaimExpiring,
    NewPoolMatch,
}

impl NotifType {
    fn as_str(self) -> &'static str {
        match self {
            NotifType::PoolEndingSoon => "pool_ending_soon",
            NotifType::PoolResolved => "pool_resolved",
            NotifType::ClaimExpiring => "claim_expiring",
            NotifType::NewPoolMatch => "new_pool_match",
        }
    }
}

/// A single notification row.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct NotificationRow {
    pub id: i64,
    pub user_address: String,
    pub notif_type: String,
    pub title: String,
    pub message: String,
    pub pool_id: Option<i64>,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

/// Insert a notification for `user_address`, silently skipping if an
/// identical `(user, pool, type)` notification already exists.
async fn create_notification(
    pool: &PgPool,
    user_address: &str,
    notif_type: NotifType,
    title: &str,
    message: &str,
    pool_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO notifications (user_address, notif_type, title, message, pool_id)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (user_address, pool_id, notif_type) DO NOTHING
        "#,
    )
    .bind(user_address)
    .bind(notif_type.as_str())
    .bind(title)
    .bind(message)
    .bind(pool_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// List `address`'s notifications, newest first.
pub async fn list_notifications(
    pool: &PgPool,
    address: &str,
    unread_only: bool,
    limit: i64,
    offset: i64,
) -> Result<Vec<NotificationRow>, sqlx::Error> {
    let sql = r#"
        SELECT id, user_address, notif_type, title, message, pool_id, read, created_at
        FROM notifications
        WHERE user_address = $1 AND ($2 = FALSE OR NOT read)
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
    "#;

    sqlx::query_as::<_, NotificationRow>(sql)
        .bind(address)
        .bind(unread_only)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
}

/// Count `address`'s unread notifications (for a badge count).
pub async fn count_unread(pool: &PgPool, address: &str) -> Result<i64, sqlx::Error> {
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_address = $1 AND NOT read")
            .bind(address)
            .fetch_one(pool)
            .await?;
    Ok(count.0)
}

/// Mark specific notification `ids` as read, or every notification for
/// `address` when `ids` is `None`. Returns the number of rows updated.
pub async fn mark_read(
    pool: &PgPool,
    address: &str,
    ids: Option<&[i64]>,
) -> Result<u64, sqlx::Error> {
    let result = match ids {
        Some(ids) => {
            sqlx::query(
                "UPDATE notifications SET read = TRUE WHERE user_address = $1 AND id = ANY($2)",
            )
            .bind(address)
            .bind(ids)
            .execute(pool)
            .await?
        }
        None => {
            sqlx::query("UPDATE notifications SET read = TRUE WHERE user_address = $1")
                .bind(address)
                .execute(pool)
                .await?
        }
    };

    Ok(result.rows_affected())
}

/// Interests `address` has registered (matched against pool category + tags
/// for "new pool matching your interests" alerts).
pub async fn get_user_interests(pool: &PgPool, address: &str) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT interest FROM user_interests WHERE user_address = $1 ORDER BY interest")
            .bind(address)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().map(|(i,)| i).collect())
}

/// Replace `address`'s interest list wholesale.
pub async fn set_user_interests(
    pool: &PgPool,
    address: &str,
    interests: &[String],
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM user_interests WHERE user_address = $1")
        .bind(address)
        .execute(&mut *tx)
        .await?;

    for interest in interests {
        sqlx::query(
            "INSERT INTO user_interests (user_address, interest) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(address)
        .bind(interest)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Request body for `PUT /api/v1/users/:address/interests`.
#[derive(Debug, Deserialize)]
pub struct SetInterestsRequest {
    pub interests: Vec<String>,
}

// ── Sweep: generates notifications from current pool/prediction state ────────

#[derive(Debug, Default, Serialize)]
pub struct SweepSummary {
    pub pools_ending_soon: usize,
    pub pools_resolved: usize,
    pub claims_expiring: usize,
    pub new_pool_matches: usize,
}

#[derive(FromRow)]
struct PoolRef {
    pool_id: i64,
    name: String,
}

#[derive(FromRow)]
struct UserAddr {
    user_address: String,
}

/// Scan current pool/prediction state and create any notifications that are
/// now due. Safe to call on a fixed interval — every insert is deduplicated
/// by the unique `(user_address, pool_id, notif_type)` index.
#[instrument(skip(pool), name = "notifications.run_sweep")]
pub async fn run_notification_sweep(pool: &PgPool) -> Result<SweepSummary, sqlx::Error> {
    let mut summary = SweepSummary::default();

    // 1. Pools ending soon → notify everyone with a stake in them.
    let ending_soon: Vec<PoolRef> = sqlx::query_as(&format!(
        r#"
        SELECT pool_id, name FROM pools
        WHERE state = 'active'
          AND end_time BETWEEN NOW() AND NOW() + INTERVAL '{POOL_ENDING_SOON_WINDOW}'
        "#
    ))
    .fetch_all(pool)
    .await?;

    for p in &ending_soon {
        let users: Vec<UserAddr> = sqlx::query_as(
            "SELECT DISTINCT user_address FROM predictions WHERE pool_id = $1",
        )
        .bind(p.pool_id)
        .fetch_all(pool)
        .await?;

        for u in &users {
            create_notification(
                pool,
                &u.user_address,
                NotifType::PoolEndingSoon,
                "Pool ending soon",
                &format!("\"{}\" closes within the hour.", p.name),
                p.pool_id,
            )
            .await?;
            summary.pools_ending_soon += 1;
        }
    }

    // 2. Resolved pools → notify everyone who predicted on them.
    let resolved: Vec<PoolRef> = sqlx::query_as(
        "SELECT pool_id, name FROM pools WHERE state = 'settled' AND result IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    for p in &resolved {
        let users: Vec<UserAddr> = sqlx::query_as(
            "SELECT DISTINCT user_address FROM predictions WHERE pool_id = $1",
        )
        .bind(p.pool_id)
        .fetch_all(pool)
        .await?;

        for u in &users {
            create_notification(
                pool,
                &u.user_address,
                NotifType::PoolResolved,
                "Pool resolved",
                &format!("\"{}\" has been resolved. Check your result.", p.name),
                p.pool_id,
            )
            .await?;
            summary.pools_resolved += 1;
        }
    }

    // 3. Claim windows expiring soon → notify unclaimed winners.
    let expiring: Vec<PoolRef> = sqlx::query_as(&format!(
        r#"
        SELECT pool_id, name FROM pools
        WHERE state = 'settled'
          AND resolved_at IS NOT NULL
          AND resolved_at + (claim_window_seconds || ' seconds')::interval
              BETWEEN NOW() AND NOW() + INTERVAL '{CLAIM_EXPIRING_WINDOW}'
        "#
    ))
    .fetch_all(pool)
    .await?;

    for p in &expiring {
        let winners: Vec<UserAddr> = sqlx::query_as(
            r#"
            SELECT DISTINCT p.user_address
            FROM predictions p
            JOIN pools pl ON pl.pool_id = p.pool_id
            WHERE p.pool_id = $1
              AND NOT p.claimed
              AND pl.result IS NOT NULL AND pl.result ~ '^\d+$'
              AND pl.result::int = p.outcome
            "#,
        )
        .bind(p.pool_id)
        .fetch_all(pool)
        .await?;

        for u in &winners {
            create_notification(
                pool,
                &u.user_address,
                NotifType::ClaimExpiring,
                "Claim window closing soon",
                &format!("Your winnings from \"{}\" must be claimed soon.", p.name),
                p.pool_id,
            )
            .await?;
            summary.claims_expiring += 1;
        }
    }

    // 4. New pools matching a user's registered interests.
    #[derive(FromRow)]
    struct NewPool {
        pool_id: i64,
        name: String,
        category: String,
        tags: Vec<String>,
    }
    let new_pools: Vec<NewPool> = sqlx::query_as(&format!(
        r#"
        SELECT pool_id, name, category, tags FROM pools
        WHERE created_at > NOW() - INTERVAL '{NEW_POOL_LOOKBACK}'
        "#
    ))
    .fetch_all(pool)
    .await?;

    for p in &new_pools {
        let mut interests = p.tags.clone();
        interests.push(p.category.clone());

        let matched_users: Vec<UserAddr> = sqlx::query_as(
            "SELECT DISTINCT user_address FROM user_interests WHERE interest = ANY($1)",
        )
        .bind(&interests)
        .fetch_all(pool)
        .await?;

        for u in &matched_users {
            create_notification(
                pool,
                &u.user_address,
                NotifType::NewPoolMatch,
                "New pool matching your interests",
                &format!("\"{}\" was just created in {}.", p.name, p.category),
                p.pool_id,
            )
            .await?;
            summary.new_pool_matches += 1;
        }
    }

    info!(
        pools_ending_soon = summary.pools_ending_soon,
        pools_resolved = summary.pools_resolved,
        claims_expiring = summary.claims_expiring,
        new_pool_matches = summary.new_pool_matches,
        "notification sweep complete"
    );

    Ok(summary)
}

/// Run [`run_notification_sweep`] on a fixed interval until the process exits.
/// Errors are logged and skipped rather than aborting the loop — a single
/// failed sweep shouldn't take the background worker down.
pub async fn run_sweep_loop(pool: PgPool, interval: std::time::Duration) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        if let Err(error) = run_notification_sweep(&pool).await {
            warn!(error = %error, "notification sweep failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notif_type_strings_match_migration_check_constraint() {
        assert_eq!(NotifType::PoolEndingSoon.as_str(), "pool_ending_soon");
        assert_eq!(NotifType::PoolResolved.as_str(), "pool_resolved");
        assert_eq!(NotifType::ClaimExpiring.as_str(), "claim_expiring");
        assert_eq!(NotifType::NewPoolMatch.as_str(), "new_pool_match");
    }
}
