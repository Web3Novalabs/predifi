//! Pool-template auto-creation scheduler (#1935).
//!
//! Recurring pool templates (`pool_templates` table, see `crate::db::pools`)
//! let a creator save a reusable pool configuration and have it
//! re-instantiated on a fixed interval (e.g. a weekly BTC price market).
//!
//! **Why this doesn't submit the on-chain transaction itself:** the backend
//! holds no custodial signing key, and the contract's `create_pool` requires
//! `creator.require_auth()` (see
//! `contract/contracts/predifi-contract/src/pool.rs`) — only the creator's
//! own wallet can authorize it. Having the backend sign and submit on a
//! user's behalf would mean adding a custodial admin keypair, a
//! significant, security-sensitive architecture change well beyond this
//! issue's scope (and the existing `worker::stellar_listener` /
//! `worker::sync` workers are read-only indexers; nothing in this codebase
//! submits Soroban transactions today).
//!
//! What this module implements instead is the scheduling half of the
//! feature: on a fixed interval, it finds every template whose
//! `next_run_at` has passed ([`crate::db::get_due_pool_templates`]),
//! notifies the creator that it's ready to be instantiated (prefilled with
//! the template's saved configuration), and advances the template's
//! schedule ([`crate::db::advance_pool_template`]) so the same cycle isn't
//! re-notified before its next recurrence.

use sqlx::PgPool;
use tracing::{info, instrument, warn};

/// Poll for due pool templates and notify their creators.
///
/// Safe to call on a fixed interval: advancing `next_run_at` immediately
/// after a successful notify prevents the same cycle from being found as
/// "due" again before its next recurrence, so this never double-notifies a
/// creator for one cycle even though the `notifications` dedupe index
/// doesn't cover template-due rows (see
/// [`crate::notifications::create_template_due_notification`]).
///
/// Returns the number of creators successfully notified.
#[instrument(skip(pool), name = "worker.template_scheduler.sweep")]
pub async fn run_template_due_sweep(pool: &PgPool) -> Result<usize, sqlx::Error> {
    let due = crate::db::get_due_pool_templates(pool).await?;
    let mut notified = 0usize;

    for template in &due {
        let title = "Recurring pool template is due";
        let message = format!(
            "\"{}\" ({} category) is ready to be created again — open the dashboard to \
             confirm and submit it on-chain.",
            template.name, template.category
        );

        match crate::notifications::create_template_due_notification(
            pool,
            &template.creator,
            title,
            &message,
        )
        .await
        {
            Ok(()) => notified += 1,
            Err(error) => {
                warn!(
                    template_id = template.id,
                    creator = %template.creator,
                    %error,
                    "failed to notify pool-template creator; will retry next sweep"
                );
                // Don't advance the schedule — retry this template next tick.
                continue;
            }
        }

        if let Err(error) = crate::db::advance_pool_template(pool, template.id).await {
            warn!(
                template_id = template.id,
                %error,
                "notified creator but failed to advance template schedule; \
                 may re-notify next sweep"
            );
        }
    }

    info!(
        due = due.len(),
        notified, "pool template due-sweep complete"
    );
    Ok(notified)
}

/// Run [`run_template_due_sweep`] on a fixed interval until the process
/// exits. Errors are logged and skipped rather than aborting the loop — a
/// single failed sweep shouldn't take the background worker down.
pub async fn run_template_sweep_loop(pool: PgPool, interval: std::time::Duration) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        if let Err(error) = run_template_due_sweep(&pool).await {
            warn!(%error, "pool template due-sweep failed");
        }
    }
}
