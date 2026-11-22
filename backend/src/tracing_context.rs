//! Helpers for propagating [`tracing`] span context across threads and tasks.
//!
//! Tokio tasks and OS threads do not automatically inherit the caller's active
//! span. Use the helpers in this module when spawning background work so logs
//! and traces remain correlated with the request or parent operation that
//! triggered them.
//!
//! # Correlation ID propagation
//!
//! The central concept is a **correlation ID** — a stable string (typically a
//! UUID v4) that identifies a logical request chain even when work is split
//! across multiple tasks, threads, or microservices.
//!
//! ```text
//! HTTP request (correlation_id = "abc-123")
//!       │
//!       ├── tokio::spawn (inherits "abc-123" via spawn())
//!       │         └── DB query span → logs contain correlation_id = "abc-123"
//!       │
//!       └── spawn_blocking (inherits "abc-123" via spawn_blocking())
//!                   └── CPU-heavy work → logs contain correlation_id = "abc-123"
//! ```
//!
//! The ID of the request being served is held in a Tokio task-local (set by
//! [`crate::request_logger::LoggingLayer`]) so code with no access to the
//! request — repository functions, outbound HTTP clients — can read it with
//! [`current_correlation_id`] and forward it with [`propagate_correlation_id`].
//! Every `spawn*` helper below carries it into the new task or thread.
//!
//! # Per-module log levels
//!
//! Log levels are controlled via `RUST_LOG` at startup and the
//! `PATCH /api/v1/admin/log-level` endpoint at runtime, both of which accept
//! per-module directives (see [`crate::telemetry`]). The recommended
//! production setting is:
//!
//! ```text
//! RUST_LOG=info,predifi_backend::request_logger=warn,predifi_backend::worker=debug
//! ```
//!
//! All workers started with [`spawn_worker`] inherit a named root span that
//! identifies them in the trace backend.

use std::future::Future;
use std::thread::JoinHandle as ThreadJoinHandle;

use tokio::task::JoinHandle as TaskJoinHandle;
use tracing::{field, info_span, Instrument, Span};
use uuid::Uuid;

// ── Correlation ID ────────────────────────────────────────────────────────────

/// Header used to receive and return the correlation ID.
pub const CORRELATION_ID_HEADER: &str = "x-correlation-id";

/// Request extension carrying the correlation ID, for handlers that prefer
/// `Extension<CorrelationId>` over the task-local.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrelationId(pub String);

tokio::task_local! {
    static CORRELATION_ID: String;
}

/// The correlation ID of the request or job running on this task, if any.
pub fn current_correlation_id() -> Option<String> {
    CORRELATION_ID.try_with(|id| id.clone()).ok()
}

/// Run `future` with `correlation_id` as the task's current correlation ID.
pub async fn with_correlation_id<F: Future>(correlation_id: String, future: F) -> F::Output {
    CORRELATION_ID.scope(correlation_id, future).await
}

/// Attach the current correlation ID (if any) to an outbound HTTP request so
/// downstream services can join their logs to ours.
pub fn propagate_correlation_id(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match current_correlation_id() {
        Some(id) => request.header(CORRELATION_ID_HEADER, id),
        None => request,
    }
}

/// Generate a new correlation ID (UUID v4).
///
/// Use this at the entry point of a request or job to produce a stable ID
/// that can be threaded through all downstream spans and log lines.
pub fn new_correlation_id() -> String {
    Uuid::new_v4().to_string()
}

/// Record a correlation ID on the *current* span.
///
/// Call this early in a request handler or background job after calling
/// [`new_correlation_id`]:
///
/// ```rust,ignore
/// let corr = tracing_context::new_correlation_id();
/// tracing_context::record_correlation_id(&corr);
/// ```
pub fn record_correlation_id(correlation_id: &str) {
    Span::current().record("correlation_id", correlation_id);
}

/// Record a user/wallet address on the *current* span.
pub fn record_user_address(address: &str) {
    Span::current().record("user_address", address);
}

/// Record a pool ID on the *current* span.
pub fn record_pool_id(pool_id: u64) {
    Span::current().record("pool_id", pool_id);
}

// ── Span construction helpers ─────────────────────────────────────────────────

/// Create a request-scoped root span with pre-allocated fields for the
/// standard PrediFi context set.
///
/// The returned span has `correlation_id`, `user_address`, and `pool_id`
/// fields reserved so they can be filled in later via [`record_correlation_id`]
/// and friends without being dropped by the subscriber.
pub fn request_span(method: &str, path: &str) -> Span {
    info_span!(
        "http.request",
        http.method = %method,
        http.route = %path,
        http.target = field::Empty,
        correlation_id = field::Empty,
        user_address = field::Empty,
        pool_id = field::Empty,
    )
}

/// Create a named worker span with a correlation ID baked in.
///
/// Use this for long-lived background workers so that every log line they
/// produce includes the worker name in the `span` field.
pub fn worker_span(worker: &str) -> Span {
    worker_span_with_id(worker, &new_correlation_id())
}

fn worker_span_with_id(worker: &str, correlation_id: &str) -> Span {
    info_span!(
        "worker",
        worker = %worker,
        correlation_id = %correlation_id,
    )
}

// ── Task / thread spawning ────────────────────────────────────────────────────

/// Spawn a Tokio task that inherits the caller's current span.
///
/// This is the preferred way to spawn short-lived async tasks that should be
/// correlated with the originating request or job.
pub fn spawn<F>(future: F) -> TaskJoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let span = tracing::Span::current();
    match current_correlation_id() {
        Some(id) => tokio::spawn(with_correlation_id(id, future).instrument(span)),
        None => tokio::spawn(future.instrument(span)),
    }
}

/// Spawn a long-lived background worker with a dedicated root span.
///
/// The worker gets its own named span and a fresh correlation ID so it is
/// easily identifiable in traces, but it does **not** inherit the caller's
/// span (which would be a short-lived request span).
pub fn spawn_worker<F>(worker: &'static str, future: F) -> TaskJoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let correlation_id = new_correlation_id();
    let span = worker_span_with_id(worker, &correlation_id);
    tokio::spawn(with_correlation_id(correlation_id, future).instrument(span))
}

/// Run blocking work on Tokio's blocking thread pool with the caller's span.
///
/// This bridges the gap between async and blocking code while preserving
/// the current trace context.
pub fn spawn_blocking<F, R>(f: F) -> TaskJoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let span = tracing::Span::current();
    let correlation_id = current_correlation_id();
    tokio::task::spawn_blocking(move || {
        let _guard = span.enter();
        run_with_optional_id(correlation_id, f)
    })
}

/// Spawn an OS thread that inherits the caller's current span.
///
/// Use for FFI or blocking system calls that cannot run on Tokio's blocking
/// pool.
pub fn spawn_thread<F, R>(f: F) -> ThreadJoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let span = tracing::Span::current();
    let correlation_id = current_correlation_id();
    std::thread::spawn(move || {
        let _guard = span.enter();
        run_with_optional_id(correlation_id, f)
    })
}

fn run_with_optional_id<F: FnOnce() -> R, R>(correlation_id: Option<String>, f: F) -> R {
    match correlation_id {
        Some(id) => CORRELATION_ID.sync_scope(id, f),
        None => f(),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;
    use tracing::Level;

    fn init_test_subscriber() {
        static INIT: OnceLock<()> = OnceLock::new();
        INIT.get_or_init(|| {
            let _ = tracing_subscriber::fmt()
                .with_max_level(Level::TRACE)
                .with_test_writer()
                .try_init();
        });
    }

    #[test]
    fn new_correlation_id_returns_uuid_string() {
        let id = new_correlation_id();
        // UUID v4 is 36 characters with dashes: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
        assert_eq!(id.len(), 36, "correlation ID must be a UUID v4: {id}");
        assert_eq!(id.chars().filter(|&c| c == '-').count(), 4);
    }

    #[test]
    fn two_correlation_ids_are_unique() {
        let a = new_correlation_id();
        let b = new_correlation_id();
        assert_ne!(a, b, "two generated IDs must be different");
    }

    #[tokio::test]
    async fn spawn_preserves_caller_span() {
        init_test_subscriber();

        let parent = tracing::info_span!("test_parent");
        let _guard = parent.enter();
        let parent_id = tracing::Span::current().id();

        let child_id = spawn(async { tracing::Span::current().id() })
            .await
            .expect("spawned task should complete");

        assert_eq!(child_id, parent_id);
    }

    #[tokio::test]
    async fn spawn_worker_creates_named_span() {
        init_test_subscriber();

        let worker_id = spawn_worker("test_worker", async {
            tracing::Span::current().metadata().map(|meta| meta.name())
        })
        .await
        .expect("worker task should complete");

        assert_eq!(worker_id, Some("worker"));
    }

    #[tokio::test]
    async fn spawn_blocking_preserves_caller_span() {
        init_test_subscriber();

        let parent = tracing::info_span!("blocking_parent");
        let _guard = parent.enter();
        let parent_id = tracing::Span::current().id();

        let child_id = spawn_blocking(|| tracing::Span::current().id())
            .await
            .expect("blocking task should complete");

        assert_eq!(child_id, parent_id);
    }

    #[test]
    fn spawn_thread_preserves_caller_span() {
        init_test_subscriber();

        let parent = tracing::info_span!("thread_parent");
        let _guard = parent.enter();
        let parent_id = tracing::Span::current().id();

        let child_id = spawn_thread(|| tracing::Span::current().id())
            .join()
            .expect("thread should complete");

        assert_eq!(child_id, parent_id);
    }

    #[tokio::test]
    async fn correlation_id_is_absent_outside_a_scope() {
        assert_eq!(current_correlation_id(), None);
    }

    #[tokio::test]
    async fn spawn_carries_correlation_id_into_child_task() {
        let child = with_correlation_id("corr-1".into(), async {
            spawn(async { current_correlation_id() }).await.unwrap()
        })
        .await;
        assert_eq!(child.as_deref(), Some("corr-1"));
    }

    #[tokio::test]
    async fn spawn_blocking_and_thread_carry_correlation_id() {
        let (blocking, thread) = with_correlation_id("corr-2".into(), async {
            let blocking = spawn_blocking(current_correlation_id).await.unwrap();
            let thread = spawn_thread(current_correlation_id).join().unwrap();
            (blocking, thread)
        })
        .await;
        assert_eq!(blocking.as_deref(), Some("corr-2"));
        assert_eq!(thread.as_deref(), Some("corr-2"));
    }

    #[tokio::test]
    async fn spawn_worker_gets_its_own_correlation_id() {
        let id = with_correlation_id("request-id".into(), async {
            spawn_worker("w", async { current_correlation_id() })
                .await
                .unwrap()
        })
        .await
        .expect("worker has an id");
        assert_ne!(id, "request-id", "workers must not inherit a request's id");
        assert_eq!(id.len(), 36);
    }

    #[test]
    fn propagate_correlation_id_sets_header_inside_scope() {
        let client = reqwest::Client::new();
        let without = propagate_correlation_id(client.get("http://localhost/"))
            .build()
            .unwrap();
        assert!(without.headers().get(CORRELATION_ID_HEADER).is_none());

        let with = CORRELATION_ID.sync_scope("corr-3".into(), || {
            propagate_correlation_id(client.get("http://localhost/"))
                .build()
                .unwrap()
        });
        assert_eq!(with.headers().get(CORRELATION_ID_HEADER).unwrap(), "corr-3");
    }

    #[test]
    fn request_span_has_expected_metadata() {
        init_test_subscriber();
        let span = request_span("GET", "/api/v1/pools");
        assert_eq!(span.metadata().map(|m| m.name()), Some("http.request"));
    }

    #[test]
    fn worker_span_has_expected_metadata() {
        init_test_subscriber();
        let span = worker_span("price_fetcher");
        assert_eq!(span.metadata().map(|m| m.name()), Some("worker"));
    }
}
