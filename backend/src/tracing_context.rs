//! Helpers for propagating [`tracing`] span context across threads and tasks.
//!
//! Tokio tasks and OS threads do not automatically inherit the caller's active
//! span. Use the helpers in this module when spawning background work so logs
//! and traces remain correlated with the request or parent operation that
//! triggered them.

use std::future::Future;
use std::thread::JoinHandle as ThreadJoinHandle;

use tokio::task::JoinHandle as TaskJoinHandle;
use tracing::Instrument;

/// Spawn a Tokio task that inherits the caller's current span.
pub fn spawn<F>(future: F) -> TaskJoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let span = tracing::Span::current();
    tokio::spawn(future.instrument(span))
}

/// Spawn a long-lived background worker with a dedicated root span.
pub fn spawn_worker<F>(worker: &'static str, future: F) -> TaskJoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let span = tracing::info_span!("worker", worker);
    tokio::spawn(future.instrument(span))
}

/// Run blocking work on Tokio's blocking thread pool with the caller's span.
pub fn spawn_blocking<F, R>(f: F) -> TaskJoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let span = tracing::Span::current();
    tokio::task::spawn_blocking(move || {
        let _guard = span.enter();
        f()
    })
}

/// Spawn an OS thread that inherits the caller's current span.
pub fn spawn_thread<F, R>(f: F) -> ThreadJoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let span = tracing::Span::current();
    std::thread::spawn(move || {
        let _guard = span.enter();
        f()
    })
}

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
}
