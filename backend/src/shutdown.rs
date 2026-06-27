//! Graceful shutdown coordination.
//!
//! This module isolates the operating-system signal handling and timeout
//! helpers that drive [`crate::server::run`]'s drain procedure.  Splitting the
//! logic out of `server.rs` keeps the dependencies on `tokio::signal`
//! localised and makes the shutdown primitives trivially testable.
//!
//! # Behaviour
//!
//! 1. [`wait_for_signal`] resolves the next time the process receives a
//!    termination request:
//!    - `SIGINT` (Ctrl+C, Kubernetes / Docker stop in some configurations),
//!    - `SIGTERM` (`kubectl delete pod`, the default Kubernetes stop signal),
//!    - `SIGHUP` (Unix reload signal; ignored on non-Unix targets).
//!
//!    On non-Unix targets only Ctrl+C is observed; the Unix-only signals
//!    degrade to `std::future::pending` so the code still compiles but those
//!    signals do not trigger shutdown.
//!
//! 2. [`with_shutdown_timeout`] races a future against a wall-clock deadline
//!    and logs whether the future completed cleanly or had to be abandoned.
//!    It is used in [`crate::server::run`] to bound how long the HTTP server
//!    is allowed to spend draining in-flight requests before the database
//!    pool and background workers are forcibly closed.

use std::future::Future;
use std::time::Duration;
use tracing::{info, warn};

/// Resolve as soon as the process receives a termination request.
///
/// The signals observed are:
/// - **SIGINT** — Ctrl+C in a terminal, or `docker stop` on some setups.
/// - **SIGTERM** — Kubernetes pod termination, the canonical "stop" signal.
/// - **SIGHUP** — Unix reload signal; treated as a stop request for our
///   purposes (the process is not long-lived in a deployment sense so we
///   simply shut down).
///
/// Each signal handler is installed independently.  If installation of any
/// one of them fails (extremely unusual on a healthy process but possible
/// in sandboxed containers), the corresponding `select!` arm is wired to
/// [`std::future::pending`] so it never resolves, while the remaining
/// successfully-installed arms still drive shutdown.  We never short-circuit
/// on a single install failure: doing so would risk missing the canonical
/// Kubernetes stop signal if Ctrl+C happened to be unavailable in the
/// runtime environment.
///
/// On non-Unix platforms only `SIGINT` is registered.
#[cfg(unix)]
pub async fn wait_for_signal() {
    // Build the three arms: a missing handler becomes a future that never
    // resolves, leaving the surviving handlers in charge of triggering the
    // `select!`.  No early-return paths are taken because a single failed
    // installation must never prevent the other signals from working.
    let mut terminate_signal = match tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate(),
    ) {
        Ok(signal) => Some(signal),
        Err(error) => {
            warn!(error = %error, "failed to install SIGTERM handler; skipping");
            None
        }
    };

    let mut hangup_signal = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()) {
        Ok(signal) => Some(signal),
        Err(error) => {
            warn!(error = %error, "failed to install SIGHUP handler; skipping");
            None
        }
    };

    let ctrl_c_block = async {
        match tokio::signal::ctrl_c().await {
            Ok(()) => info!("received Ctrl+C, beginning graceful shutdown"),
            Err(error) => warn!(error = %error, "Ctrl+C handler failed; relying on SIGTERM/SIGHUP"),
        }
    };

    let terminate_block = async {
        match terminate_signal.as_mut() {
            Some(signal) => {
    async fn wait_sigterm() {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
                info!("received SIGTERM, beginning graceful shutdown");
            }
            Err(error) => {
                warn!(error = %error, "failed to install SIGTERM handler; skipping");
                std::future::pending::<()>().await;
            }
        }
    }

    let hangup_block = async {
        match hangup_signal.as_mut() {
            Some(signal) => {
    async fn wait_sighup() {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()) {
            Ok(mut signal) => {
                signal.recv().await;
                info!("received SIGHUP, beginning graceful shutdown");
            }
            Err(error) => {
                warn!(error = %error, "failed to install SIGHUP handler; skipping");
                std::future::pending::<()>().await;
            }
        }
    }

    async fn wait_ctrl_c() {
        if let Err(error) = tokio::signal::ctrl_c().await {
            warn!(error = %error, "Ctrl+C handler failed");
        }
        info!("received Ctrl+C, beginning graceful shutdown");
    }

    tokio::select! {
        _ = wait_ctrl_c() => {},
        _ = wait_sigterm() => {},
        _ = wait_sighup() => {},
    }
}

/// Non-Unix implementation: only Ctrl+C is observably delivered to a Rust
/// process running on Windows.  This function therefore resolves when the
/// user presses Ctrl+C in the controlling terminal.
#[cfg(not(unix))]
pub async fn wait_for_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(_) => {
            info!("received Ctrl+C, beginning graceful shutdown");
        }
        Err(error) => {
            warn!(error = %error, "failed to install Ctrl+C handler; shutting down anyway");
        }
    if let Err(error) = tokio::signal::ctrl_c().await {
        warn!(error = %error, "Ctrl+C handler failed; shutting down anyway");
        return;
    }
    info!("received Ctrl+C, beginning graceful shutdown");
}

/// Run `fut` with a wall-clock deadline.
///
/// - If `fut` completes within `timeout`, the helper logs a success message
///   and returns.
/// - If `timeout` elapses first, the helper logs a warning and returns;
///   `fut` is dropped at that point, which aborts any in-progress work.
///
/// The helper exists so that the various shutdown phases (HTTP drain, DB
/// pool close, worker abort) can each be capped independently, with a clear
/// log line indicating which phase exceeded its budget.
pub async fn with_shutdown_timeout<F>(timeout: Duration, name: &str, fut: F)
where
    F: Future<Output = ()>,
{
    match tokio::time::timeout(timeout, fut).await {
        Ok(()) => {
            info!(
                component = name,
                timeout_secs = timeout.as_secs(),
                "shutdown phase completed cleanly"
            );
        }
        Err(_) => {
            warn!(
                component = name,
                timeout_secs = timeout.as_secs(),
                "shutdown phase timed out; some operations may be cut short"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// `with_shutdown_timeout` resolves cleanly when the future finishes
    /// inside the deadline.
    #[tokio::test]
    async fn shutdown_timeout_returns_ok_when_future_completes() {
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = done.clone();
        with_shutdown_timeout(Duration::from_secs(1), "unit", async move {
            done_clone.store(true, Ordering::SeqCst);
        })
        .await;
        assert!(done.load(Ordering::SeqCst));
    }

    /// `with_shutdown_timeout` does not panic and does not block forever when
    /// the future would take longer than the deadline.
    #[tokio::test]
    async fn shutdown_timeout_returns_after_deadline_when_future_is_slow() {
        // The inner future sleeps for 2 s; the timeout deadline is 100 ms.
        // The helper must return after ~100 ms, well before the 2-second sleep
        // would complete.
        let start = tokio::time::Instant::now();
        with_shutdown_timeout(Duration::from_millis(100), "slow-unit", async {
            tokio::time::sleep(Duration::from_secs(2)).await;
        let start = tokio::time::Instant::now();
        with_shutdown_timeout(Duration::from_millis(50), "slow-unit", async {
            tokio::time::sleep(Duration::from_secs(10)).await;
        })
        .await;
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(100),
            "helper should have waited at least the deadline (got {elapsed:?})"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            elapsed >= Duration::from_millis(50),
            "helper should have waited at least the deadline (got {elapsed:?})"
        );
        assert!(
            elapsed < Duration::from_secs(1),
            "helper should not have waited for the full future (got {elapsed:?})"
        );
    }

    /// `with_shutdown_timeout` resolves promptly when the inner future
    /// completes near-instantly.  This guarantees nothing in the helper
    /// itself adds latency.
    #[tokio::test]
    async fn shutdown_timeout_returns_promptly_when_future_is_instant() {
        let start = tokio::time::Instant::now();
        with_shutdown_timeout(Duration::from_secs(2), "instant", async {}).await;
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "with_shutdown_timeout should return without waiting for a fast future"
        );
    }
}
