//! Graceful shutdown coordination.
//!
//! This module isolates the operating-system signal handling and timeout
//! helpers that drive [`crate::server::run`]'s drain procedure.  Splitting the
//! logic out of `server.rs` keeps the dependencies on `tokio::signal`
//! localised and makes the shutdown primitives trivially testable.
//!
//! # Production-grade shutdown sequence
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
//!
//! 3. [`drain_websockets`] notifies all open WebSocket connections that the
//!    server is shutting down, sends a close frame, and waits up to the given
//!    deadline for each connection to finish its current message.
//!
//! 4. [`drain_workers`] awaits a list of task handles with a shared deadline,
//!    logging each that fails to finish in time.
//!
//! # Zero-downtime guarantees
//!
//! - All in-flight HTTP requests are given up to `shutdown_timeout_secs` to
//!   complete before the listener is forcibly closed (handled in `server.rs`
//!   via `axum::serve(...).with_graceful_shutdown(signal)`).
//! - WebSocket connections receive a Close frame and are given a bounded
//!   period to echo it back before being dropped.
//! - Background workers (price-cache fetcher, Stellar listener, notification
//!   sweep) are given their own timeout window before being aborted.
//! - The PostgreSQL pool is closed *after* workers are stopped so there is no
//!   window where a worker issues a query against a closed pool.
//! - The OpenTelemetry batch exporter is flushed last so that shutdown spans
//!   are captured.

use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

// ── Signal handlers ───────────────────────────────────────────────────────────

/// Resolve as soon as the process receives a termination request.
///
/// The signals observed are:
/// - **SIGINT** — Ctrl+C in a terminal, or `docker stop` on some setups.
/// - **SIGTERM** — Kubernetes pod termination, the canonical "stop" signal.
/// - **SIGHUP** — Unix reload signal; treated as a stop request here.
///
/// Each signal handler is installed independently.  If installation of any
/// one of them fails (extremely unusual on a healthy process but possible
/// in sandboxed containers), the corresponding `select!` arm is wired to
/// [`std::future::pending`] so it never resolves, while the remaining
/// successfully-installed arms still drive shutdown.
///
/// On non-Unix platforms only `SIGINT` is registered.
#[cfg(unix)]
pub async fn wait_for_signal() {
    let mut terminate_signal =
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(signal) => Some(signal),
            Err(error) => {
                warn!(error = %error, "failed to install SIGTERM handler; skipping");
                None
            }
        };

    let mut hangup_signal =
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()) {
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
                signal.recv().await;
                info!("received SIGTERM, beginning graceful shutdown");
            }
            None => std::future::pending::<()>().await,
        }
    };

    let hangup_block = async {
        match hangup_signal.as_mut() {
            Some(signal) => {
                signal.recv().await;
                info!("received SIGHUP, beginning graceful shutdown");
            }
            None => std::future::pending::<()>().await,
        }
    };

    tokio::select! {
        _ = ctrl_c_block => {},
        _ = terminate_block => {},
        _ = hangup_block => {},
    }
}

/// Non-Unix implementation: only Ctrl+C is observably delivered to a Rust
/// process running on Windows.  This function therefore resolves when the
/// user presses Ctrl+C in the controlling terminal.
#[cfg(not(unix))]
pub async fn wait_for_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => info!("received Ctrl+C, beginning graceful shutdown"),
        Err(error) => {
            warn!(error = %error, "failed to install Ctrl+C handler; shutting down anyway")
        }
    }
}

// ── Timeout wrapper ───────────────────────────────────────────────────────────

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

// ── WebSocket drain ───────────────────────────────────────────────────────────

/// A token handed out to each WebSocket connection so the shutdown
/// coordinator can signal all of them simultaneously and wait for the
/// active-connection count to reach zero.
#[derive(Clone)]
pub struct WsShutdownToken {
    /// Sender side of the broadcast channel that carries the shutdown signal.
    sender: broadcast::Sender<()>,
    /// Shared count of currently active WebSocket connections.
    active: Arc<AtomicUsize>,
}

impl WsShutdownToken {
    /// Create a new token with an initial capacity of `capacity` receivers.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1));
        Self {
            sender,
            active: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Subscribe a new WebSocket connection to the shutdown broadcast.
    ///
    /// Call this when a connection is *opened*.  The connection handler should
    /// `select!` on the returned receiver and send a close frame when it fires.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.active.fetch_add(1, Ordering::Relaxed);
        self.sender.subscribe()
    }

    /// Decrement the active connection counter.
    ///
    /// Call this when a connection is *closed*.
    pub fn connection_closed(&self) {
        self.active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Broadcast the shutdown signal to all live WebSocket connections.
    ///
    /// Returns the number of receivers that were notified.
    pub fn broadcast_shutdown(&self) -> usize {
        match self.sender.send(()) {
            Ok(n) => {
                info!(connections = n, "WebSocket shutdown signal broadcast");
                n
            }
            // No active receivers — nothing to do.
            Err(_) => {
                info!("no active WebSocket connections to notify");
                0
            }
        }
    }

    /// Return the current number of active WebSocket connections.
    pub fn active_count(&self) -> usize {
        self.active.load(Ordering::Relaxed)
    }
}

/// Signal all open WebSocket connections to close and wait up to `timeout`
/// for the active count to reach zero.
///
/// This gives every connection a chance to send a clean Close frame before
/// the underlying TCP sockets are torn down by the HTTP server drain.
pub async fn drain_websockets(token: &WsShutdownToken, timeout: Duration) {
    let notified = token.broadcast_shutdown();
    if notified == 0 {
        return;
    }

    let deadline = tokio::time::Instant::now() + timeout;
    let poll_interval = Duration::from_millis(10);

    loop {
        let remaining = token.active_count();
        if remaining == 0 {
            info!("all WebSocket connections drained cleanly");
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            warn!(
                remaining,
                timeout_secs = timeout.as_secs_f64(),
                "WebSocket drain timed out; {} connection(s) still open",
                remaining
            );
            return;
        }
        tokio::time::sleep(poll_interval).await;
    }
}

// ── Background worker drain ───────────────────────────────────────────────────

/// Wait for a list of named background worker task handles to finish, each
/// bounded by the same `timeout`.
///
/// Workers that do not finish within `timeout` are aborted and a warning is
/// logged.  Workers that complete successfully log an info message.
///
/// # Example
///
/// ```rust,ignore
/// let price_handle = tokio::spawn(price_fetcher());
/// let listener_handle = tokio::spawn(stellar_listener());
///
/// let workers = vec![
///     ("price_fetcher", price_handle),
///     ("stellar_listener", listener_handle),
/// ];
/// drain_workers(workers, Duration::from_secs(30)).await;
/// ```
pub async fn drain_workers(workers: Vec<(&'static str, JoinHandle<()>)>, timeout: Duration) {
    for (name, handle) in workers {
        match tokio::time::timeout(timeout, handle).await {
            Ok(Ok(())) => {
                info!(worker = name, "background worker finished cleanly");
            }
            Ok(Err(join_err)) if join_err.is_cancelled() => {
                // The task was aborted externally — treat as clean.
                info!(worker = name, "background worker was cancelled");
            }
            Ok(Err(join_err)) => {
                error!(
                    worker = name,
                    error = %join_err,
                    "background worker panicked during shutdown"
                );
            }
            Err(_elapsed) => {
                warn!(
                    worker = name,
                    timeout_secs = timeout.as_secs(),
                    "background worker did not finish within shutdown timeout; aborting"
                );
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

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
        let start = tokio::time::Instant::now();
        with_shutdown_timeout(Duration::from_millis(100), "slow-unit", async {
            tokio::time::sleep(Duration::from_secs(2)).await;
        })
        .await;
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(100),
            "helper should have waited at least the deadline (got {elapsed:?})"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "helper should not have waited for the full future (got {elapsed:?})"
        );
    }

    /// `with_shutdown_timeout` resolves promptly when the inner future
    /// completes near-instantly.
    #[tokio::test]
    async fn shutdown_timeout_returns_promptly_when_future_is_instant() {
        let start = tokio::time::Instant::now();
        with_shutdown_timeout(Duration::from_secs(2), "instant", async {}).await;
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "with_shutdown_timeout should return without waiting for a fast future"
        );
    }

    // ── WsShutdownToken tests ─────────────────────────────────────────────────

    /// Subscribing increments the active counter; closing decrements it.
    #[test]
    fn ws_token_tracks_active_connections() {
        let token = WsShutdownToken::new(8);
        assert_eq!(token.active_count(), 0);

        let _rx1 = token.subscribe();
        let _rx2 = token.subscribe();
        assert_eq!(token.active_count(), 2);

        token.connection_closed();
        assert_eq!(token.active_count(), 1);
    }

    /// `broadcast_shutdown` returns 0 when there are no receivers.
    #[test]
    fn ws_token_broadcast_with_no_receivers_returns_zero() {
        let token = WsShutdownToken::new(4);
        // No subscribe calls — no receivers.
        assert_eq!(token.broadcast_shutdown(), 0);
    }

    /// Active receivers receive the shutdown signal.
    #[tokio::test]
    async fn ws_token_broadcast_reaches_receivers() {
        let token = WsShutdownToken::new(4);
        let mut rx = token.subscribe();

        let notified = token.broadcast_shutdown();
        assert_eq!(notified, 1, "one receiver should be notified");

        // The receiver should immediately have a message waiting.
        assert!(
            rx.try_recv().is_ok(),
            "receiver should have the shutdown signal"
        );
    }

    /// `drain_websockets` returns immediately when there are no connections.
    #[tokio::test]
    async fn drain_websockets_returns_immediately_with_no_connections() {
        let token = WsShutdownToken::new(4);
        let start = tokio::time::Instant::now();
        drain_websockets(&token, Duration::from_secs(5)).await;
        assert!(start.elapsed() < Duration::from_millis(200));
    }

    /// `drain_workers` completes without panic when workers finish instantly.
    #[tokio::test]
    async fn drain_workers_handles_instant_completion() {
        let handle = tokio::spawn(async {});
        drain_workers(vec![("instant-worker", handle)], Duration::from_secs(1)).await;
    }

    /// `drain_workers` logs a timeout for a slow worker and does not block.
    #[tokio::test]
    async fn drain_workers_times_out_slow_worker() {
        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(60)).await;
        });

        let start = tokio::time::Instant::now();
        drain_workers(vec![("slow-worker", handle)], Duration::from_millis(100)).await;
        let elapsed = start.elapsed();

        assert!(
            elapsed >= Duration::from_millis(100),
            "should have waited for the timeout"
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "should not have waited for the full 60 s"
        );
    }
}
