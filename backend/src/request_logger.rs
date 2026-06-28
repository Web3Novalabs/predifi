//! # Request Logging Middleware
//!
//! This module provides a Tower-compatible middleware layer for Axum that
//! logs the HTTP method, path, response status code, and total request
//! duration for every incoming request.
//!
//! ## How it works
//!
//! In Axum (and the wider Tower ecosystem), a **middleware** is a piece of
//! code that sits between the server and your route handlers.  Every request
//! passes *through* the middleware before reaching your handler, and every
//! response passes back *through* it on the way out.  This gives us a single
//! place to observe both the incoming request and the final response.
//!
//! ```text
//!  HTTP request
//!       │
//!       ▼
//! ┌─────────────────┐
//! │ LoggingMiddleware│  ← records method + path, starts timer
//! └────────┬────────┘
//!          │
//!          ▼
//!   Route handler        ← your normal business logic
//!          │
//!          ▼
//! ┌─────────────────┐
//! │ LoggingMiddleware│  ← records status + elapsed time, prints log line
//! └────────┬────────┘
//!          │
//!          ▼
//!   HTTP response
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use axum::{Router, routing::get};
//! use request_logger::logging::LoggingLayer;
//!
//! let app = Router::new()
//!     .route("/", get(|| async { "hello" }))
//!     .layer(LoggingLayer);   // <-- attach the middleware
//! ```
//!
//! ## Output format
//!
//! ```text
//! [REQ] GET /api/users → 200 OK (4ms)
//! [REQ] POST /api/orders → 422 Unprocessable Entity (12ms)
//! ```

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use axum::http::{Request, Response};
use tower::{Layer, Service};
use tracing::{error, info, info_span, Instrument};

use crate::metrics::SharedMetrics;

// Layer

/// A Tower [`Layer`] that wraps every service with [`LoggingService`].
///
/// Attach this to your Axum router with `.layer(LoggingLayer::new())` or
/// `.layer(LoggingLayer::with_metrics(metrics))` to also record request
/// latency histograms.
#[derive(Clone, Default)]
pub struct LoggingLayer {
    metrics: Option<SharedMetrics>,
}

impl LoggingLayer {
    /// Create a logging layer that emits structured logs only.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a logging layer that also records Prometheus latency metrics.
    pub fn with_metrics(metrics: SharedMetrics) -> Self {
        Self {
            metrics: Some(metrics),
        }
    }
}

impl<S> Layer<S> for LoggingLayer {
    type Service = LoggingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LoggingService {
            inner,
            metrics: self.metrics.clone(),
        }
    }
}

// Service

/// The actual middleware service produced by [`LoggingLayer`].
///
/// `S` is the *inner* service — i.e. whatever comes after this middleware in
/// the stack (usually your route handlers).  `LoggingService` wraps `S` and
/// intercepts every request/response pair so it can emit a log line.
#[derive(Clone)]
pub struct LoggingService<S> {
    inner: S,
    metrics: Option<SharedMetrics>,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for LoggingService<S>
where
    // S must itself be a Service that accepts the same request type.
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    // The inner service's future must be sendable across threads (Axum
    // requires this so it can run handlers on its async runtime).
    S::Future: Send + 'static,
    // S::Error must also be Send so the logging future can propagate it.
    S::Error: Send + 'static,
{
    // We pass through the inner service's response and error types unchanged.
    type Response = S::Response;
    type Error = S::Error;

    // Our future type — a heap-allocated future (`Box<dyn Future>`) because
    // we need to store state (start time, method, path) across the await
    // point where the inner handler runs.
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    /// Tower calls this to ask "are you ready to handle a request?".
    /// We just delegate to the inner service.
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    /// This is called once per request.  We:
    /// 1. Snapshot the method and path *before* forwarding.
    /// 2. Start a timer.
    /// 3. `await` the inner service (where the real work happens).
    /// 4. After the response comes back, print the log line.
    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        // Extract the information we want to log.  We do this *before*
        // passing the request to the inner service because `call` consumes
        // the request — we can't inspect it afterwards.
        let method = req.method().to_string();
        let path = req.uri().path().to_string();

        // Record when the request arrived so we can measure latency.
        let start = Instant::now();

        // All per-request context lives on the span so the JSON formatter
        // emits it once under "span" without duplicating it in "fields".
        let span = info_span!(
            "http.request",
            http.method = %method,
            http.route = %path,
        );

        // Forward the request to the inner service and get back a Future
        // that will eventually produce a Response.
        let metrics = self.metrics.clone();
        let inner_future = self.inner.call(req);

        Box::pin(async move {
            let result = inner_future.await;
            let elapsed = start.elapsed();
            let elapsed_ms = elapsed.as_millis();

            match &result {
                Ok(response) => {
                    let status = response.status();
                    let status_label = status.as_u16().to_string();
                    info!(
                        http.status_code = status.as_u16(),
                        http.duration_ms = elapsed_ms,
                        "request complete"
                    );
                    if let Some(metrics) = metrics {
                        metrics
                            .http_request_duration_seconds
                            .with_label_values(&[&method, &path, &status_label])
                            .observe(elapsed.as_secs_f64());
                    }
                }
                Err(_) => {
                    error!(
                        http.duration_ms = elapsed_ms,
                        "request failed"
                    );
                    if let Some(metrics) = metrics {
                        metrics
                            .http_request_duration_seconds
                            .with_label_values(&[&method, &path, "error"])
                            .observe(elapsed.as_secs_f64());
                    }
                }
            }

            result
        }
        .instrument(span))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, routing::get, Router};
    use http::Request;
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn logging_layer_records_request_latency_metric() {
        let metrics = Arc::new(crate::metrics::Metrics::new().expect("metrics"));
        let app = Router::new()
            .route("/ping", get(|| async { "pong" }))
            .layer(LoggingLayer::with_metrics(metrics.clone()));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/ping")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), http::StatusCode::OK);

        let text = metrics.gather_text().expect("metrics text");
        assert!(
            text.contains("app_http_request_duration_seconds"),
            "latency histogram must be recorded, got: {text}"
        );
    }
}
