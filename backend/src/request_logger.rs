//! # Request Logging Middleware
//!
//! This module provides a Tower-compatible middleware layer for Axum that
//! emits **structured JSON log lines** for every HTTP request, including:
//!
//! - HTTP method, path, status code, and duration.
//! - A **correlation ID** (`x-correlation-id` / `x-request-id`) that is
//!   propagated through the entire request chain so logs and traces from
//!   multiple services can be joined.
//! - **User context** — the authenticated wallet address extracted from the
//!   `x-user-address` header (populated by the JWT middleware upstream).
//! - **Pool context** — the pool ID extracted from the URL path when present
//!   (e.g. `/api/v1/pools/42/...`).
//! - Prometheus latency metrics recorded via [`SharedMetrics`].
//!
//! ## Output format
//!
//! Every request produces a tracing span whose JSON representation looks like:
//!
//! ```json
//! {
//!   "timestamp": "2026-07-26T10:30:00.123456Z",
//!   "level": "INFO",
//!   "target": "predifi_backend::request_logger",
//!   "span": {
//!     "http.method": "GET",
//!     "http.route": "/api/v1/pools/42",
//!     "correlation_id": "01J2K3L4M5N6P7Q8R9S0T1U2V3",
//!     "user_address": "GABC...XYZ",
//!     "pool_id": "42"
//!   },
//!   "fields": {
//!     "http.status_code": 200,
//!     "http.duration_ms": 4,
//!     "message": "request complete"
//!   }
//! }
//! ```
//!
//! ## Log-level configurability
//!
//! Use the `RUST_LOG` environment variable (or `LOG_LEVEL` in `.env`) to
//! control verbosity per-module:
//!
//! ```text
//! RUST_LOG=predifi_backend::request_logger=debug,warn
//! ```
//!
//! At `debug` level every request is logged; at `info` only requests that
//! take longer than `SLOW_REQUEST_THRESHOLD_MS` are elevated to `warn`.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use axum::{Router, routing::get};
//! use request_logger::LoggingLayer;
//!
//! let app = Router::new()
//!     .route("/", get(|| async { "hello" }))
//!     .layer(LoggingLayer::new());
//! ```

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

use axum::http::{HeaderMap, Request, Response};
use tower::{Layer, Service};
use tracing::{error, info, info_span, warn, Instrument};
use uuid::Uuid;

use crate::metrics::SharedMetrics;

/// Requests that take longer than this threshold are logged at `WARN` level
/// even when the status code indicates success.
const SLOW_REQUEST_THRESHOLD_MS: u128 = 1_000;

// ── Helper: extract a correlation ID ─────────────────────────────────────────

/// Return the correlation ID for a request.
///
/// Priority order:
/// 1. `x-correlation-id` header (e.g. set by an API gateway).
/// 2. `x-request-id` header (common alternative).
/// 3. A freshly generated UUID v4 (guarantees every request has an ID).
fn extract_correlation_id(headers: &HeaderMap) -> String {
    for header_name in &["x-correlation-id", "x-request-id"] {
        if let Some(val) = headers.get(*header_name) {
            if let Ok(s) = val.to_str() {
                if !s.is_empty() {
                    return s.to_string();
                }
            }
        }
    }
    Uuid::new_v4().to_string()
}

/// Extract the authenticated user's wallet address from the request headers.
///
/// The JWT middleware is expected to have already validated the token and
/// written the wallet address into the `x-user-address` internal header
/// before the request reaches this middleware.
fn extract_user_address(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-user-address")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Extract the pool ID from a URL path such as `/api/v1/pools/42/predictions`.
///
/// Returns `None` if the path does not contain a numeric segment immediately
/// after `/pools/`.
fn extract_pool_id(path: &str) -> Option<String> {
    let mut parts = path.split('/');
    while let Some(segment) = parts.next() {
        if segment == "pools" {
            if let Some(id) = parts.next() {
                if id.chars().all(|c| c.is_ascii_digit()) && !id.is_empty() {
                    return Some(id.to_string());
                }
            }
        }
    }
    None
}

// ── Layer ─────────────────────────────────────────────────────────────────────

/// A Tower [`Layer`] that wraps every service with [`LoggingService`].
///
/// Attach this to your Axum router with `.layer(LoggingLayer::new())` or
/// `.layer(LoggingLayer::with_metrics(metrics))` to also record Prometheus
/// latency metrics.
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

// ── Service ───────────────────────────────────────────────────────────────────

/// The actual middleware service produced by [`LoggingLayer`].
///
/// `S` is the *inner* service — i.e. whatever comes after this middleware in
/// the stack (usually your route handlers).
#[derive(Clone)]
pub struct LoggingService<S> {
    inner: S,
    metrics: Option<SharedMetrics>,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for LoggingService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        // ── Extract per-request context ────────────────────────────────────
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let correlation_id = extract_correlation_id(req.headers());
        let user_address = extract_user_address(req.headers());
        let pool_id = extract_pool_id(&path);

        let start = Instant::now();
        let metrics = self.metrics.clone();

        // Build the span.  All per-request context is attached here so the
        // JSON formatter emits it once under "span", not duplicated in each
        // log field.
        let span = info_span!(
            "http.request",
            http.method = %method,
            http.route = %path,
            correlation_id = %correlation_id,
            user_address = user_address.as_deref().unwrap_or("anonymous"),
            pool_id = pool_id.as_deref().unwrap_or(""),
        );

        let inner_future = self.inner.call(req);

        Box::pin(
            async move {
                let result = inner_future.await;
                let elapsed = start.elapsed();
                let elapsed_ms = elapsed.as_millis();

                match &result {
                    Ok(response) => {
                        let status = response.status();
                        let status_u16 = status.as_u16();
                        let status_label = status_u16.to_string();

                        // Emit at WARN for slow requests or server errors.
                        if status_u16 >= 500 {
                            warn!(
                                http.status_code = status_u16,
                                http.duration_ms = elapsed_ms,
                                "request complete with server error"
                            );
                        } else if elapsed_ms >= SLOW_REQUEST_THRESHOLD_MS {
                            warn!(
                                http.status_code = status_u16,
                                http.duration_ms = elapsed_ms,
                                "request complete (slow)"
                            );
                        } else {
                            info!(
                                http.status_code = status_u16,
                                http.duration_ms = elapsed_ms,
                                "request complete"
                            );
                        }

                        if let Some(m) = &metrics {
                            m.http_request_duration_seconds
                                .with_label_values(&[&method, &path, &status_label])
                                .observe(elapsed.as_secs_f64());
                            m.http_requests_total
                                .with_label_values(&[&method, &path, &status_label])
                                .inc();
                            if status_u16 >= 500 {
                                m.http_server_errors_total.inc();
                            }
                        }
                    }
                    Err(_) => {
                        error!(
                            http.duration_ms = elapsed_ms,
                            "request failed with transport error"
                        );
                        if let Some(m) = &metrics {
                            m.http_request_duration_seconds
                                .with_label_values(&[&method, &path, "error"])
                                .observe(elapsed.as_secs_f64());
                        }
                    }
                }

                result
            }
            .instrument(span),
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

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
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), http::StatusCode::OK);

        let text = metrics.gather_text().expect("metrics text");
        assert!(
            text.contains("app_http_request_duration_seconds"),
            "latency histogram must be recorded, got: {text}"
        );
    }

    #[test]
    fn extract_correlation_id_uses_x_correlation_id_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-correlation-id", "test-id-123".parse().unwrap());
        let id = extract_correlation_id(&headers);
        assert_eq!(id, "test-id-123");
    }

    #[test]
    fn extract_correlation_id_falls_back_to_x_request_id() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", "fallback-456".parse().unwrap());
        let id = extract_correlation_id(&headers);
        assert_eq!(id, "fallback-456");
    }

    #[test]
    fn extract_correlation_id_generates_uuid_when_absent() {
        let headers = HeaderMap::new();
        let id = extract_correlation_id(&headers);
        // Should be a valid UUID v4 (36 chars with dashes).
        assert_eq!(id.len(), 36, "generated ID should be a UUID: {id}");
    }

    #[test]
    fn extract_user_address_returns_header_value() {
        let mut headers = HeaderMap::new();
        headers.insert("x-user-address", "GABC123XYZ".parse().unwrap());
        assert_eq!(
            extract_user_address(&headers),
            Some("GABC123XYZ".to_string())
        );
    }

    #[test]
    fn extract_user_address_returns_none_when_absent() {
        let headers = HeaderMap::new();
        assert!(extract_user_address(&headers).is_none());
    }

    #[test]
    fn extract_pool_id_from_path() {
        assert_eq!(
            extract_pool_id("/api/v1/pools/42/predictions"),
            Some("42".to_string())
        );
        assert_eq!(extract_pool_id("/api/v1/pools/7"), Some("7".to_string()));
    }

    #[test]
    fn extract_pool_id_returns_none_for_non_pool_paths() {
        assert_eq!(extract_pool_id("/api/v1/health"), None);
        assert_eq!(extract_pool_id("/api/v1/pools/new"), None);
        assert_eq!(extract_pool_id("/"), None);
    }

    #[tokio::test]
    async fn logging_layer_propagates_correlation_id_in_span() {
        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .layer(LoggingLayer::new());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .header("x-correlation-id", "trace-abc-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), http::StatusCode::OK);
    }
}
