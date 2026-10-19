//! # Request Logging Middleware
//!
//! This module provides a Tower-compatible middleware layer for Axum that
//! emits **structured JSON log lines** for every HTTP request, including:
//!
//! - HTTP method, path, status code, and duration.
//! - A **correlation ID** (`x-correlation-id` / `x-request-id`) that is
//!   propagated through the entire request chain so logs and traces from
//!   multiple services can be joined. The ID is validated (untrusted input
//!   must not be able to forge log lines), echoed back on the response in
//!   `x-correlation-id`, stored as a [`CorrelationId`] request extension, and
//!   made available to everything the handler awaits or spawns via
//!   [`crate::tracing_context::current_correlation_id`].
//! - **User context** — the wallet address from a *verified* access-token
//!   `Authorization: Bearer` header (when the layer is built with
//!   [`LoggingLayer::with_jwt`]), redacted. Unauthenticated or invalid tokens
//!   are logged as `anonymous`; client-supplied identity headers are ignored.
//! - **Pool context** — the pool ID extracted from the URL path when present
//!   (e.g. `/api/v1/pools/42/...`).
//! - Prometheus latency metrics recorded via [`SharedMetrics`], labelled by a
//!   normalised route (`/api/v1/pools/:id`) so IDs and addresses cannot blow
//!   up label cardinality.
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
//!   "message": "request complete",
//!   "http.status_code": 200,
//!   "http.duration_ms": 4,
//!   "span": {
//!     "name": "http.request",
//!     "http.method": "GET",
//!     "http.route": "/api/v1/pools/:id",
//!     "http.target": "/api/v1/pools/42",
//!     "correlation_id": "6f1c0e9a-3c1b-4d55-9a57-2b8f2f4f6d10",
//!     "user_address": "GABCDEFG...",
//!     "pool_id": 42
//!   },
//!   "spans": [ ... ]
//! }
//! ```
//!
//! ## Log-level configurability
//!
//! Use the `RUST_LOG` environment variable (or the admin log-level endpoint
//! at runtime) to control verbosity per-module:
//!
//! ```text
//! RUST_LOG=warn,predifi_backend::request_logger=info
//! ```
//!
//! Successful requests log at `info`; server errors and requests slower than
//! `SLOW_REQUEST_THRESHOLD_MS` are elevated to `warn`. Setting
//! `predifi_backend::request_logger=warn` keeps only the problem requests.
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

use std::sync::Arc;

use axum::http::{HeaderMap, HeaderValue, Request, Response};
use tower::{Layer, Service};
use tracing::{error, info, warn, Instrument};
use uuid::Uuid;

use crate::metrics::SharedMetrics;
use crate::tracing_context::{CorrelationId, CORRELATION_ID_HEADER};

/// Longest correlation ID accepted from a client.
const MAX_CORRELATION_ID_LEN: usize = 128;

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
///
/// Header values that fail [`is_valid_correlation_id`] are ignored so a
/// client cannot inject newlines, JSON, or megabytes of text into our logs.
fn extract_correlation_id(headers: &HeaderMap) -> String {
    for header_name in [CORRELATION_ID_HEADER, "x-request-id"] {
        if let Some(val) = headers.get(header_name) {
            if let Ok(s) = val.to_str() {
                if is_valid_correlation_id(s) {
                    return s.to_string();
                }
            }
        }
    }
    Uuid::new_v4().to_string()
}

/// A correlation ID is 1–128 characters of `[A-Za-z0-9._:-]`.
fn is_valid_correlation_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_CORRELATION_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
}

/// Replace path segments that identify a resource with placeholders, so the
/// route can be used as a low-cardinality metric label.
///
/// `/api/v1/pools/42/predictions` → `/api/v1/pools/:id/predictions`;
/// Stellar account/contract addresses become `:address`.
pub fn normalize_route(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if !segment.is_empty() && segment.bytes().all(|b| b.is_ascii_digit()) {
                ":id"
            } else if is_stellar_address(segment) {
                ":address"
            } else if Uuid::parse_str(segment).is_ok() {
                ":uuid"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn is_stellar_address(segment: &str) -> bool {
    segment.len() == 56
        && matches!(segment.as_bytes()[0], b'G' | b'C' | b'M')
        && segment
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

// ── Helpers ────────────────────────────────────────────────────────────────────

const REDACTED_ADDRESS_SUFFIX: &str = "...";

/// Redact a wallet address for log output, keeping only the first 8 characters
/// so traces can still be correlated within a single request chain without
/// exposing enough of the address to identify an account from logs alone.
pub fn redact_address(address: &str) -> String {
    if address.is_empty() {
        return address.to_string();
    }
    let prefix: String = address.chars().take(8).collect();
    format!("{prefix}{REDACTED_ADDRESS_SUFFIX}")
}

/// Secret and key version used to verify access tokens for log context.
#[derive(Clone)]
struct JwtContext {
    secret: Arc<str>,
    key_version: u64,
}

/// Extract the authenticated user's wallet address from a verified access
/// token in the `Authorization` header.
///
/// Returns `None` when there is no token or it fails verification — an
/// unverified `sub` would let any client choose whose name its requests are
/// logged under.
fn extract_user_address(headers: &HeaderMap, jwt: Option<&JwtContext>) -> Option<String> {
    let jwt = jwt?;
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(crate::jwt::extract_bearer_token)?;
    crate::jwt::verify_jwt_token_strict(token, &jwt.secret, "access", jwt.key_version)
        .ok()
        .map(|claims| claims.sub)
        .filter(|sub| !sub.is_empty())
}

/// Extract the pool ID from a URL path such as `/api/v1/pools/42/predictions`.
///
/// Returns `None` if the path does not contain a numeric segment immediately
/// after `/pools/`.
fn extract_pool_id(path: &str) -> Option<String> {
    let mut parts = path.split('/');
    while let Some(segment) = parts.next() {
        if segment == "pools" || segment == "markets" {
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
    jwt: Option<JwtContext>,
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
            jwt: None,
        }
    }

    /// Verify `Authorization: Bearer` access tokens with `secret` so the
    /// authenticated wallet can be attached to each request's log context.
    pub fn with_jwt(mut self, secret: &str, key_version: u64) -> Self {
        self.jwt = Some(JwtContext {
            secret: Arc::from(secret),
            key_version,
        });
        self
    }
}

impl<S> Layer<S> for LoggingLayer {
    type Service = LoggingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LoggingService {
            inner,
            metrics: self.metrics.clone(),
            jwt: self.jwt.clone(),
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
    jwt: Option<JwtContext>,
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

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        // ── Extract per-request context ────────────────────────────────────
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let route = normalize_route(&path);
        let correlation_id = extract_correlation_id(req.headers());
        let user_address = extract_user_address(req.headers(), self.jwt.as_ref())
            .map(|a| redact_address(&a))
            .unwrap_or_else(|| "anonymous".to_string());
        let pool_id = extract_pool_id(&path).and_then(|id| id.parse::<u64>().ok());

        req.extensions_mut()
            .insert(CorrelationId(correlation_id.clone()));

        let start = Instant::now();
        let metrics = self.metrics.clone();

        // Build the span. All per-request context is attached here so the
        // JSON formatter emits it once under "span", not duplicated in each
        // log field. Handlers can add context later through
        // `tracing_context::record_*` because the fields are pre-declared.
        let span = crate::tracing_context::request_span(&method, &route);
        span.record("http.target", path.as_str());
        span.record("correlation_id", correlation_id.as_str());
        span.record("user_address", user_address.as_str());
        if let Some(pool_id) = pool_id {
            span.record("pool_id", pool_id);
        }

        let inner_future = {
            let _entered = span.enter();
            self.inner.call(req)
        };
        let header_value = HeaderValue::from_str(&correlation_id).ok();

        let future = async move {
            let mut result = inner_future.await;
            let elapsed = start.elapsed();
            let elapsed_ms = elapsed.as_millis();

            match &mut result {
                Ok(response) => {
                    if let Some(value) = header_value {
                        response.headers_mut().insert(CORRELATION_ID_HEADER, value);
                    }

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
                            .with_label_values(&[&method, &route, &status_label])
                            .observe(elapsed.as_secs_f64());
                        m.http_requests_total
                            .with_label_values(&[&method, &route, &status_label])
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
                            .with_label_values(&[&method, &route, "error"])
                            .observe(elapsed.as_secs_f64());
                    }
                }
            }

            result
        };

        Box::pin(crate::tracing_context::with_correlation_id(
            correlation_id,
            future.instrument(span),
        ))
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

    const SECRET: &str = "predifi-dev-secret-do-not-use-in-production-32";

    fn jwt() -> JwtContext {
        JwtContext {
            secret: Arc::from(SECRET),
            key_version: 0,
        }
    }

    fn bearer(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {token}").parse().unwrap(),
        );
        headers
    }

    #[test]
    fn extract_user_address_uses_verified_access_token() {
        let token = crate::jwt::sign_jwt_for_test("GABC123XYZ", SECRET);
        assert_eq!(
            extract_user_address(&bearer(&token), Some(&jwt())),
            Some("GABC123XYZ".to_string())
        );
    }

    #[test]
    fn extract_user_address_rejects_unverified_identity() {
        // Wrong secret.
        let forged =
            crate::jwt::sign_jwt_for_test("GEVIL", "another-secret-of-at-least-32-bytes!!");
        assert!(extract_user_address(&bearer(&forged), Some(&jwt())).is_none());
        // Spoofed header is ignored.
        let mut headers = HeaderMap::new();
        headers.insert("x-user-address", "GSPOOF".parse().unwrap());
        assert!(extract_user_address(&headers, Some(&jwt())).is_none());
        // No JWT context configured.
        let token = crate::jwt::sign_jwt_for_test("GABC", SECRET);
        assert!(extract_user_address(&bearer(&token), None).is_none());
    }

    #[test]
    fn extract_user_address_returns_none_when_absent() {
        let headers = HeaderMap::new();
        assert!(extract_user_address(&headers, Some(&jwt())).is_none());
    }

    #[test]
    fn invalid_correlation_ids_are_replaced() {
        for bad in ["", "has space", "line\nbreak", "{\"json\":1}"] {
            let mut headers = HeaderMap::new();
            if let Ok(value) = bad.parse() {
                headers.insert("x-correlation-id", value);
            }
            let id = extract_correlation_id(&headers);
            assert_ne!(id, bad);
            assert_eq!(id.len(), 36, "falls back to a UUID for {bad:?}");
        }
        let mut headers = HeaderMap::new();
        headers.insert("x-correlation-id", "a".repeat(129).parse().unwrap());
        assert_eq!(extract_correlation_id(&headers).len(), 36);
    }

    #[test]
    fn normalize_route_replaces_ids_and_addresses() {
        let address = format!("G{}", "A".repeat(55));
        assert_eq!(normalize_route("/api/v1/pools/42"), "/api/v1/pools/:id");
        assert_eq!(
            normalize_route(&format!("/api/v1/users/{address}/predictions")),
            "/api/v1/users/:address/predictions"
        );
        assert_eq!(
            normalize_route("/api/v1/x/6f1c0e9a-3c1b-4d55-9a57-2b8f2f4f6d10"),
            "/api/v1/x/:uuid"
        );
        assert_eq!(normalize_route("/health"), "/health");
        assert_eq!(normalize_route("/"), "/");
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
        assert_eq!(
            response.headers().get("x-correlation-id").unwrap(),
            "trace-abc-123",
            "the correlation ID must be echoed back to the caller"
        );
    }

    #[tokio::test]
    async fn logging_layer_generates_and_exposes_correlation_id() {
        use axum::Extension;

        let app = Router::new()
            .route(
                "/id",
                get(|Extension(id): Extension<CorrelationId>| async move {
                    let task_local = crate::tracing_context::current_correlation_id();
                    assert_eq!(task_local.as_deref(), Some(id.0.as_str()));
                    id.0
                }),
            )
            .layer(LoggingLayer::new());

        let response = app
            .oneshot(Request::builder().uri("/id").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let header = response
            .headers()
            .get("x-correlation-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        assert_eq!(header.len(), 36);
        assert_eq!(body, header.as_bytes(), "handler sees the same ID");
    }

    #[tokio::test]
    async fn logging_layer_labels_metrics_by_normalized_route() {
        let metrics = Arc::new(crate::metrics::Metrics::new().expect("metrics"));
        let app = Router::new()
            .route("/pools/:id", get(|| async { "ok" }))
            .layer(LoggingLayer::with_metrics(metrics.clone()));
        for id in [1, 2, 3] {
            app.clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/pools/{id}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
        }
        assert_eq!(
            metrics
                .http_requests_total
                .with_label_values(&["GET", "/pools/:id", "200"])
                .get(),
            3.0
        );
    }
}
