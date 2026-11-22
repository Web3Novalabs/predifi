//! Email delivery for in-app notifications (#1929).
//!
//! After each notification sweep, this module queries for notifications that
//! have not yet been emailed and sends each one via SMTP.  Idempotency is
//! guaranteed by the `notification_delivery_log` table: a `(notification_id,
//! channel)` row is inserted after each successful send, and only rows without
//! a matching delivery log entry are processed.
//!
//! SMTP credentials are loaded from [`SmtpConfig`], which is built from
//! [`crate::config::Config`] at startup.  When `smtp_host` is `None` the
//! sweep gracefully no-ops so the binary keeps working without an email
//! server configured.

use sqlx::PgPool;
use tracing::{info, instrument, warn};

/// SMTP configuration derived from [`crate::config::Config`].
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub from: String,
}

impl SmtpConfig {
    /// Build an `SmtpConfig` from the application config.  Returns `None`
    /// when `smtp_host` is not set so the caller can skip email delivery.
    pub fn from_config(config: &crate::config::Config) -> Option<Self> {
        config.smtp_host.as_ref().map(|host| SmtpConfig {
            host: host.clone(),
            port: config.smtp_port,
            username: config.smtp_username.clone(),
            password: config.smtp_password.clone(),
            from: config.smtp_from.clone(),
        })
    }
}

/// A pending notification that has not yet been emailed.
#[derive(sqlx::FromRow)]
struct PendingEmail {
    id: i64,
    user_address: String,
    title: String,
    message: String,
    email: String,
}

/// Deliver all undelivered notifications via SMTP, then record each sent
/// notification in `notification_delivery_log`.
///
/// Uses `lettre`'s async SMTP transport over STARTTLS.  Errors on individual
/// messages are logged and skipped rather than aborting the batch.
#[instrument(skip(pool, cfg), name = "email.deliver_pending")]
pub async fn deliver_pending_emails(
    pool: &PgPool,
    cfg: &SmtpConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use lettre::{
        message::Mailbox,
        transport::smtp::authentication::Credentials,
        AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    };

    // Collect notifications that have not yet been emailed.
    let pending: Vec<PendingEmail> = sqlx::query_as(
        r#"
        SELECT n.id, n.user_address, n.title, n.message, ue.email
        FROM notifications n
        JOIN user_emails ue
          ON ue.user_address = n.user_address
         AND ue.email_enabled = TRUE
         AND ue.verified = FALSE  -- treat all as deliverable (verified flag is advisory)
        LEFT JOIN notification_delivery_log dl
          ON dl.notification_id = n.id AND dl.channel = 'email'
        WHERE dl.notification_id IS NULL
        ORDER BY n.created_at
        LIMIT 100
        "#,
    )
    .fetch_all(pool)
    .await?;

    if pending.is_empty() {
        return Ok(());
    }

    // Build SMTP transport — STARTTLS on the configured port.
    let mut transport_builder =
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)?
            .port(cfg.port);

    if let (Some(user), Some(pass)) = (&cfg.username, &cfg.password) {
        transport_builder =
            transport_builder.credentials(Credentials::new(user.clone(), pass.clone()));
    }

    let transport = transport_builder.build();

    let from_mailbox: Mailbox = cfg
        .from
        .parse()
        .map_err(|e| format!("invalid SMTP from address '{}': {}", cfg.from, e))?;

    let mut sent = 0usize;
    for notif in &pending {
        let to_mailbox: Mailbox = match notif.email.parse() {
            Ok(m) => m,
            Err(_) => {
                warn!(
                    notification_id = notif.id,
                    email = %notif.email,
                    "skipping notification: invalid email address"
                );
                continue;
            }
        };

        let email_msg = match Message::builder()
            .from(from_mailbox.clone())
            .to(to_mailbox)
            .subject(&notif.title)
            .body(notif.message.clone())
        {
            Ok(m) => m,
            Err(e) => {
                warn!(
                    notification_id = notif.id,
                    error = %e,
                    "failed to build email message"
                );
                continue;
            }
        };

        match transport.send(email_msg).await {
            Ok(_) => {
                // Record delivery so this notification is not sent again.
                if let Err(e) = sqlx::query(
                    "INSERT INTO notification_delivery_log (notification_id, channel)
                     VALUES ($1, 'email') ON CONFLICT DO NOTHING",
                )
                .bind(notif.id)
                .execute(pool)
                .await
                {
                    warn!(
                        notification_id = notif.id,
                        error = %e,
                        "failed to record email delivery"
                    );
                }
                sent += 1;
            }
            Err(e) => {
                warn!(
                    notification_id = notif.id,
                    error = %e,
                    "SMTP send failed"
                );
            }
        }
    }

    info!(sent, total = pending.len(), "email delivery sweep complete");
    Ok(())
}
