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
    /// A recurring pool template (#1935) is due to be instantiated again.
    TemplateDue,
}

impl NotifType {
    fn as_str(self) -> &'static str {
        match self {
            NotifType::PoolEndingSoon => "pool_ending_soon",
            NotifType::PoolResolved => "pool_resolved",
            NotifType::ClaimExpiring => "claim_expiring",
            NotifType::NewPoolMatch => "new_pool_match",
            NotifType::TemplateDue => "template_due",
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
        ON CONFLICT DO NOTHING
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

/// Insert a "recurring pool template is due" notification for `creator`
/// (#1935).
///
/// Unlike [`create_notification`], this has no real `pool_id` yet — the
/// on-chain pool hasn't been created, since the backend has no custodial
/// signing key and can't submit that transaction on the creator's behalf
/// (see `worker::template_scheduler`). The `(user_address, pool_id,
/// notif_type)` dedupe index only applies `WHERE pool_id IS NOT NULL` (see
/// migration 010), so this is instead kept idempotent by the caller
/// advancing the template's `next_run_at` right after a successful insert —
/// the same due cycle won't be found again until the next recurrence.
pub async fn create_template_due_notification(
    pool: &PgPool,
    creator: &str,
    title: &str,
    message: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO notifications (user_address, notif_type, title, message, pool_id)
        VALUES ($1, $2, $3, $4, NULL)
        "#,
    )
    .bind(creator)
    .bind(NotifType::TemplateDue.as_str())
    .bind(title)
    .bind(message)
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

/// Resolution / claim-expiry notices are only generated for pools resolved
/// within this window, so the first sweep after a deploy does not notify
/// users about every historical pool.
const RESOLVED_LOOKBACK: &str = "7 days";

/// Scan current pool/prediction state and create any notifications that are
/// now due. Safe to call on a fixed interval — every insert is deduplicated
/// by the unique `(user_address, pool_id, notif_type)` index, and each
/// category is a single set-based `INSERT … SELECT` (no per-user round trips).
/// Counts in the summary are rows actually inserted.
#[instrument(skip(pool), name = "notifications.run_sweep")]
pub async fn run_notification_sweep(pool: &PgPool) -> Result<SweepSummary, sqlx::Error> {
    let mut summary = SweepSummary::default();

    // 1. Pools ending soon → notify everyone with a stake in them, plus the
    //    pool's creator.
    summary.pools_ending_soon = sqlx::query(&format!(
        r#"
        INSERT INTO notifications (user_address, notif_type, title, message, pool_id)
        SELECT u.user_address, 'pool_ending_soon', 'Pool ending soon',
               format('"%s" closes within the hour.', p.name), p.pool_id
        FROM pools p
        CROSS JOIN LATERAL (
            SELECT DISTINCT user_address FROM predictions WHERE pool_id = p.pool_id
            UNION
            SELECT p.creator WHERE p.creator <> ''
        ) u
        WHERE p.state = 'active'
          AND p.end_time BETWEEN NOW() AND NOW() + INTERVAL '{POOL_ENDING_SOON_WINDOW}'
        ON CONFLICT DO NOTHING
        "#
    ))
    .execute(pool)
    .await?
    .rows_affected() as usize;

    // 2. Recently resolved pools → notify everyone who predicted on them, with
    //    their own outcome when it can be determined.
    summary.pools_resolved = sqlx::query(&format!(
        r#"
        INSERT INTO notifications (user_address, notif_type, title, message, pool_id)
        SELECT DISTINCT pr.user_address, 'pool_resolved', 'Pool resolved',
               format('"%s" has been resolved. Check your result.', p.name), p.pool_id
        FROM pools p
        JOIN predictions pr ON pr.pool_id = p.pool_id
        WHERE p.state = 'settled'
          AND p.result IS NOT NULL
          AND p.resolved_at IS NOT NULL
          AND p.resolved_at > NOW() - INTERVAL '{RESOLVED_LOOKBACK}'
        ON CONFLICT DO NOTHING
        "#
    ))
    .execute(pool)
    .await?
    .rows_affected() as usize;

    // 3. Claim windows expiring soon → notify unclaimed winners.
    summary.claims_expiring = sqlx::query(&format!(
        r#"
        INSERT INTO notifications (user_address, notif_type, title, message, pool_id)
        SELECT DISTINCT pr.user_address, 'claim_expiring', 'Claim window closing soon',
               format('Your winnings from "%s" must be claimed soon.', p.name), p.pool_id
        FROM pools p
        JOIN predictions pr ON pr.pool_id = p.pool_id
        WHERE p.state = 'settled'
          AND p.resolved_at IS NOT NULL
          AND p.resolved_at + (p.claim_window_seconds || ' seconds')::interval
              BETWEEN NOW() AND NOW() + INTERVAL '{CLAIM_EXPIRING_WINDOW}'
          AND NOT pr.claimed
          AND p.result IS NOT NULL AND p.result ~ '^\d+$'
          AND p.result::int = pr.outcome
        ON CONFLICT DO NOTHING
        "#
    ))
    .execute(pool)
    .await?
    .rows_affected() as usize;

    // 4. New pools matching a user's registered interests (category or tag,
    //    case-insensitive). The creator is not notified about their own pool.
    summary.new_pool_matches = sqlx::query(&format!(
        r#"
        INSERT INTO notifications (user_address, notif_type, title, message, pool_id)
        SELECT DISTINCT ui.user_address, 'new_pool_match', 'New pool matching your interests',
               format('"%s" was just created in %s.', p.name, p.category), p.pool_id
        FROM pools p
        JOIN user_interests ui
          ON ui.interest = lower(p.category)
          OR ui.interest IN (SELECT lower(t) FROM unnest(p.tags) AS t)
        WHERE p.created_at > NOW() - INTERVAL '{NEW_POOL_LOOKBACK}'
          AND ui.user_address <> p.creator
        ON CONFLICT DO NOTHING
        "#
    ))
    .execute(pool)
    .await?
    .rows_affected() as usize;

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

/// Run [`run_notification_sweep`] on a fixed interval and, when an SMTP host
/// is configured, also deliver undelivered notifications via email.
///
/// This variant is used by the server when email credentials are available.
pub async fn run_sweep_loop_with_email(
    pool: PgPool,
    interval: std::time::Duration,
    smtp_config: crate::email::SmtpConfig,
) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        if let Err(error) = run_notification_sweep(&pool).await {
            warn!(error = %error, "notification sweep failed");
            continue;
        }
        if let Err(error) = crate::email::deliver_pending_emails(&pool, &smtp_config).await {
            warn!(error = %error, "email delivery sweep failed");
        }
    }
}

/// Run [`run_notification_sweep`] on a fixed interval and deliver the
/// resulting notifications over every configured channel (email via SMTP,
/// push via the push gateway). A channel that is `None` is skipped.
pub async fn run_sweep_loop_with_delivery(
    pool: PgPool,
    interval: std::time::Duration,
    smtp_config: Option<crate::email::SmtpConfig>,
    push_config: Option<crate::push::PushConfig>,
) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        if let Err(error) = run_notification_sweep(&pool).await {
            warn!(error = %error, "notification sweep failed");
            continue;
        }
        if let Some(smtp) = &smtp_config {
            if let Err(error) = crate::email::deliver_pending_emails(&pool, smtp).await {
                warn!(error = %error, "email delivery sweep failed");
            }
        }
        if let Some(push) = &push_config {
            if let Err(error) = crate::push::deliver_pending_push(&pool, push).await {
                warn!(error = %error, "push delivery sweep failed");
            }
        }
    }
}

// ── User email registration endpoints ────────────────────────────────────────

/// User email registration / update request body.
#[derive(Debug, Deserialize)]
pub struct RegisterEmailRequest {
    pub email: String,
    pub email_enabled: Option<bool>,
}

/// Upsert a user's email address and opt-in preference.
pub async fn upsert_user_email(
    pool: &PgPool,
    address: &str,
    email: &str,
    email_enabled: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO user_emails (user_address, email, email_enabled, updated_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (user_address)
        DO UPDATE SET email = $2, email_enabled = $3, updated_at = NOW()
        "#,
    )
    .bind(address)
    .bind(email)
    .bind(email_enabled)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get the email and opt-in state for a user address.
pub async fn get_user_email(
    pool: &PgPool,
    address: &str,
) -> Result<Option<(String, bool)>, sqlx::Error> {
    let row: Option<(String, bool)> = sqlx::query_as(
        "SELECT email, email_enabled FROM user_emails WHERE user_address = $1",
    )
    .bind(address)
    .fetch_optional(pool)
    .await?;
    Ok(row)
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
