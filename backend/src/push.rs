//! Push delivery for in-app notifications.
//!
//! Mirrors [`crate::email`]: after each notification sweep, notifications that
//! have no `push` row in `notification_delivery_log` are POSTed to a push
//! gateway (FCM/APNs/Web Push relay) once per registered device token.
//! Idempotency comes from the delivery log. When no gateway is configured the
//! sweep is a no-op.
//!
//! Gateway contract — `POST <PREDIFI_PUSH_GATEWAY_URL>` with an optional
//! `Authorization: Bearer <PREDIFI_PUSH_GATEWAY_TOKEN>` header and JSON body:
//!
//! ```json
//! {"token":"…","platform":"web","title":"…","body":"…",
//!  "data":{"notification_id":1,"type":"pool_resolved","pool_id":7}}
//! ```
//!
//! A `404`/`410` response means the token is no longer valid and the
//! subscription is deleted.

use std::time::Duration;

use serde::Deserialize;
use sqlx::PgPool;
use tracing::{info, instrument, warn};

/// Only notifications newer than this are pushed, so enabling push never
/// floods devices with a historical backlog.
const PUSH_MAX_AGE: &str = "2 days";
const PUSH_BATCH_LIMIT: i64 = 100;
const MAX_PUSH_TOKEN_LENGTH: usize = 4096;
const PUSH_HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// Gateway settings derived from [`crate::config::Config`].
#[derive(Debug, Clone)]
pub struct PushConfig {
    pub gateway_url: String,
    pub gateway_token: Option<String>,
}

impl PushConfig {
    /// `None` when no gateway URL is configured (push disabled).
    pub fn from_config(config: &crate::config::Config) -> Option<Self> {
        config.push_gateway_url.as_ref().map(|url| PushConfig {
            gateway_url: url.clone(),
            gateway_token: config.push_gateway_token.clone(),
        })
    }
}

/// Request body for registering a push token.
#[derive(Debug, Deserialize)]
pub struct RegisterPushRequest {
    pub token: String,
    /// `web` (default), `ios` or `android`.
    pub platform: Option<String>,
}

/// Request body for removing a push token.
#[derive(Debug, Deserialize)]
pub struct UnregisterPushRequest {
    pub token: String,
}

/// Validate and normalise a registration request into `(token, platform)`.
pub fn validate_registration(req: &RegisterPushRequest) -> Result<(String, String), &'static str> {
    let token = validate_token(&req.token)?;
    let platform = req
        .platform
        .as_deref()
        .map(|p| p.trim().to_lowercase())
        .unwrap_or_else(|| "web".to_string());
    if !matches!(platform.as_str(), "web" | "ios" | "android") {
        return Err("platform must be one of web, ios, android");
    }
    Ok((token, platform))
}

/// Trim and bound-check a push token.
pub fn validate_token(raw: &str) -> Result<String, &'static str> {
    let token = raw.trim();
    if token.is_empty() {
        return Err("token must not be empty");
    }
    if token.len() > MAX_PUSH_TOKEN_LENGTH {
        return Err("token must be 4096 characters or fewer");
    }
    Ok(token.to_string())
}

/// Register (or refresh) a push token for `address`.
pub async fn register_subscription(
    pool: &PgPool,
    address: &str,
    token: &str,
    platform: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO push_subscriptions (user_address, token, platform)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_address, token) DO UPDATE SET platform = EXCLUDED.platform
        "#,
    )
    .bind(address)
    .bind(token)
    .bind(platform)
    .execute(pool)
    .await?;
    Ok(())
}

/// Remove a push token. Returns `true` when a subscription was deleted.
pub async fn unregister_subscription(
    pool: &PgPool,
    address: &str,
    token: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM push_subscriptions WHERE user_address = $1 AND token = $2")
        .bind(address)
        .bind(token)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Number of devices registered for `address`.
pub async fn count_subscriptions(pool: &PgPool, address: &str) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM push_subscriptions WHERE user_address = $1")
        .bind(address)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

#[derive(sqlx::FromRow)]
struct PendingPush {
    id: i64,
    notif_type: String,
    title: String,
    message: String,
    pool_id: Option<i64>,
    sub_id: i64,
    token: String,
    platform: String,
}

/// Deliver undelivered notifications to every registered device.
#[instrument(skip(pool, cfg), name = "push.deliver_pending")]
pub async fn deliver_pending_push(
    pool: &PgPool,
    cfg: &PushConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pending: Vec<PendingPush> = sqlx::query_as(&format!(
        r#"
        SELECT n.id, n.notif_type, n.title, n.message, n.pool_id,
               ps.id AS sub_id, ps.token, ps.platform
        FROM notifications n
        JOIN push_subscriptions ps ON ps.user_address = n.user_address
        LEFT JOIN notification_delivery_log dl
          ON dl.notification_id = n.id AND dl.channel = 'push'
        WHERE dl.notification_id IS NULL
          AND n.created_at > NOW() - INTERVAL '{PUSH_MAX_AGE}'
        ORDER BY n.created_at, n.id
        LIMIT {PUSH_BATCH_LIMIT}
        "#
    ))
    .fetch_all(pool)
    .await?;

    if pending.is_empty() {
        return Ok(());
    }

    let client = reqwest::Client::builder().timeout(PUSH_HTTP_TIMEOUT).build()?;
    let mut delivered_ids = std::collections::BTreeSet::new();
    let mut seen_ids = std::collections::BTreeSet::new();

    for item in &pending {
        seen_ids.insert(item.id);
        let mut request = client.post(&cfg.gateway_url).json(&serde_json::json!({
            "token": item.token,
            "platform": item.platform,
            "title": item.title,
            "body": item.message,
            "data": {
                "notification_id": item.id,
                "type": item.notif_type,
                "pool_id": item.pool_id,
            },
        }));
        if let Some(token) = &cfg.gateway_token {
            request = request.bearer_auth(token);
        }

        match request.send().await {
            Ok(response) if response.status().is_success() => {
                delivered_ids.insert(item.id);
            }
            Ok(response)
                if matches!(response.status().as_u16(), 404 | 410) =>
            {
                // Token is dead — stop pushing to it.
                if let Err(error) = sqlx::query("DELETE FROM push_subscriptions WHERE id = $1")
                    .bind(item.sub_id)
                    .execute(pool)
                    .await
                {
                    warn!(subscription_id = item.sub_id, %error, "failed to prune stale push token");
                }
            }
            Ok(response) => {
                warn!(notification_id = item.id, status = %response.status(), "push gateway rejected notification");
            }
            Err(error) => {
                warn!(notification_id = item.id, %error, "push gateway request failed");
            }
        }
    }

    // A notification with no remaining deliverable device is marked done so
    // it is not retried forever; otherwise only successes are logged.
    for id in &delivered_ids {
        if let Err(error) = sqlx::query(
            "INSERT INTO notification_delivery_log (notification_id, channel)
             VALUES ($1, 'push') ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .execute(pool)
        .await
        {
            warn!(notification_id = id, %error, "failed to record push delivery");
        }
    }

    info!(
        delivered = delivered_ids.len(),
        attempted = seen_ids.len(),
        "push delivery sweep complete"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_and_oversized_tokens() {
        assert!(validate_token("   ").is_err());
        assert!(validate_token(&"x".repeat(MAX_PUSH_TOKEN_LENGTH + 1)).is_err());
        assert_eq!(validate_token("  abc ").unwrap(), "abc");
    }

    #[test]
    fn platform_defaults_to_web_and_is_allow_listed() {
        let ok = RegisterPushRequest { token: "t".into(), platform: None };
        assert_eq!(validate_registration(&ok).unwrap().1, "web");
        let bad = RegisterPushRequest { token: "t".into(), platform: Some("symbian".into()) };
        assert!(validate_registration(&bad).is_err());
    }
}
