use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt; // provides `.oneshot()`

// Pull in the router builder from main.rs.
use crate::{build_router, price_cache::PriceCache};

/// Build a bare GET request with no body for the given path.
fn get(path: &str) -> Request<axum::body::Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .body(axum::body::Body::empty())
        .expect("failed to build request")
}

/// Read a response body all the way into a `String`.
async fn body_string(body: axum::body::Body) -> String {
    let bytes = body
        .collect()
        .await
        .expect("failed to collect body")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("body is not valid utf-8")
}

use crate::config::Config;

/// GET /must return HTTP 200.
#[tokio::test]
async fn root_returns_200() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
}

/// GET /health must return HTTP 200 with `{"status":"ok"}` in the body.
#[tokio::test]
async fn health_returns_200_with_ok_body() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/health"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"status\""),
        "body should contain a status field, got: {body}"
    );
}

/// GET /api/v1/health must return HTTP 200 from the nested v1 router.
#[tokio::test]
async fn api_v1_health_returns_200_with_versioned_body() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/api/v1/health"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"status\"") && body.contains("\"version\":\"v1\""),
        "body should contain status and version fields, got: {body}"
    );
}

/// GET /api/v1 must return HTTP 200 from the version discovery route.
#[tokio::test]
async fn api_v1_index_returns_200() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/api/v1"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
}

/// GET /api/v1/fees returns the current fee configuration.
#[tokio::test]
async fn api_v1_fees_returns_config_values() {
    let mut config = Config::default_for_test();
    config.treasury_fee_bps = 400;
    config.referral_fee_bps = 6000;

    let response = build_router(config, PriceCache::new())
        .oneshot(get("/api/v1/fees"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"treasury_fee_bps\":400"),
        "body should contain treasury_fee_bps, got: {body}"
    );
    assert!(
        body.contains("\"referral_fee_bps\":6000"),
        "body should contain referral_fee_bps, got: {body}"
    );
}

/// GET /nonexistent must return HTTP 404 (Axum's built-in fallback).
#[tokio::test]
async fn unknown_route_returns_404() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/nonexistent"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Verify the middleware does not alter the status code of a 200 response.
#[tokio::test]
async fn middleware_does_not_alter_200_status() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/health"))
        .await
        .expect("request failed");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "logging middleware must not modify the response status"
    );
}

/// Verify the middleware does not alter the status code of a 404 response.
#[tokio::test]
async fn middleware_does_not_alter_404_status() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/no-such-path"))
        .await
        .expect("request failed");

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "logging middleware must not modify 404 responses"
    );
}

/// Fire multiple requests through the same router to confirm that the
/// middleware handles repeated calls without errors or panics.
#[tokio::test]
async fn middleware_handles_multiple_requests_sequentially() {
    let paths_and_expected: &[(&str, StatusCode)] = &[
        ("/", StatusCode::OK),
        ("/health", StatusCode::OK),
        ("/missing", StatusCode::NOT_FOUND),
    ];

    for (path, expected_status) in paths_and_expected {
        let response = build_router(Config::default_for_test(), PriceCache::new())
            .oneshot(get(path))
            .await
            .expect("request failed");

        assert_eq!(
            response.status(),
            *expected_status,
            "unexpected status for {path}"
        );
    }
}

/// CORS headers must be present when a request comes from an allowed origin.
#[tokio::test]
async fn cors_allows_allowed_origin() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:5173")
                .body(axum::body::Body::empty())
                .expect("failed to build request"),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let allow_origin = response
        .headers()
        .get("access-control-allow-origin")
        .and_then(|v| v.to_str().ok());

    assert_eq!(
        allow_origin,
        Some("http://localhost:5173"),
        "CORS header should reflect the allowed origin"
    );
}

/// Preflight OPTIONS request must return 200 for allowed origins.
#[tokio::test]
async fn cors_handles_preflight_request() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/health")
                .header(header::ORIGIN, "http://localhost:5173")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .body(axum::body::Body::empty())
                .expect("failed to build request"),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
}

/// Verify that the rate limiter returns 429 Too Many Requests after exceeding the limit.
#[tokio::test]
async fn rate_limiting_returns_429_after_burst() {
    let app = build_router(Config::default_for_test(), PriceCache::new());

    // The limit is 50 requests burst.
    // We fire 50 requests which should all be 200 OK.
    for _ in 0..50 {
        let response = app.clone()
            .oneshot(get("/health"))
            .await
            .expect("request failed");
        assert_eq!(response.status(), StatusCode::OK);
    }

    // The 51st request should be rate limited.
    let response = app.oneshot(get("/health"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

/// GET /api/v1/users/:address/referrals without a DB returns 503.
#[tokio::test]
async fn user_referrals_without_db_returns_503() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/api/v1/users/GABC123/referrals"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("database not configured"),
        "body should mention database not configured, got: {body}"
    );
}
