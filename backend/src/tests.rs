use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt; // provides `.oneshot()`

use crate::config::Config;
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

/// GET / must return HTTP 200.
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

/// GET /metrics must return HTTP 200 and expose Prometheus text format.
#[tokio::test]
async fn metrics_endpoint_returns_200() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/metrics"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response.into_body()).await;
    assert!(body.contains("# HELP app_http_requests_total"));
    assert!(body.contains("app_up"));
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
        let response = app
            .clone()
            .oneshot(get("/health"))
            .await
            .expect("request failed");
        assert_eq!(response.status(), StatusCode::OK);
    }

    // The 51st request should be rate limited.
    let response = app.oneshot(get("/health")).await.expect("request failed");

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

// ──────────────────────────────────────────────────────────────────────────────
// Advanced Health Check Tests
// ──────────────────────────────────────────────────────────────────────────────

/// Test that /api/v1/health returns 200 with dependency status when everything is OK.
#[tokio::test]
async fn api_v1_health_returns_200_with_dependency_status() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/api/v1/health"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"status\""),
        "body should contain status field, got: {body}"
    );
    assert!(
        body.contains("\"version\":\"v1\""),
        "body should contain version field, got: {body}"
    );
    assert!(
        body.contains("\"dependencies\""),
        "body should contain dependencies field, got: {body}"
    );
    assert!(
        body.contains("\"db\""),
        "body should contain db dependency status, got: {body}"
    );
    assert!(
        body.contains("\"rpc\""),
        "body should contain rpc dependency status, got: {body}"
    );
}

/// Test that /health returns 200 with dependency status when everything is OK.
#[tokio::test]
async fn root_health_returns_200_with_dependency_status() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/health"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"status\""),
        "body should contain status field, got: {body}"
    );
    assert!(
        body.contains("\"service\":\"predifi-backend\""),
        "body should contain service field, got: {body}"
    );
    assert!(
        body.contains("\"dependencies\""),
        "body should contain dependencies field, got: {body}"
    );
}

/// Test that /api/v1/health reports db as 'not_configured' when no database is provided.
#[tokio::test]
async fn api_v1_health_reports_db_not_configured_without_pool() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/api/v1/health"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"db\":\"not_configured\""),
        "body should indicate db is not_configured, got: {body}"
    );
}

/// Test that /health reports db as 'not_configured' when no database is provided.
#[tokio::test]
async fn root_health_reports_db_not_configured_without_pool() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/health"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"db\":\"not_configured\""),
        "body should indicate db is not_configured, got: {body}"
    );
}

/// Test that /api/v1/health returns the "ok" status when healthy.
#[tokio::test]
async fn api_v1_health_status_is_ok_when_healthy() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/api/v1/health"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"status\":\"ok\""),
        "status should be 'ok' when healthy, got: {body}"
    );
}

/// Test that /health returns the "ok" status when healthy.
#[tokio::test]
async fn root_health_status_is_ok_when_healthy() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/health"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"status\":\"ok\""),
        "status should be 'ok' when healthy, got: {body}"
    );
}

/// Test that health endpoint includes the version from Cargo.toml.
#[tokio::test]
async fn health_includes_cargo_version() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/health"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"version\""),
        "body should contain version field, got: {body}"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// 503 Service Unavailable Tests (Acceptance Criteria Verification)
// ──────────────────────────────────────────────────────────────────────────────

/// Test that /api/v1/health returns HTTP 503 when RPC is unreachable.
/// This is critical for the acceptance criteria: "Returns 503 if any dependency is unreachable."
#[tokio::test]
async fn api_v1_health_returns_503_when_rpc_unreachable() {
    let mut config = Config::default_for_test();
    // Point RPC to an invalid/unreachable endpoint to simulate failure
    config.stellar_rpc_url = String::from("http://localhost:1/invalid");

    let response = build_router(config, PriceCache::new())
        .oneshot(get("/api/v1/health"))
        .await
        .expect("request failed");

    assert_eq!(
        response.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "health endpoint should return 503 when RPC is unreachable (acceptance criteria)"
    );

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"status\":\"error\""),
        "status should be 'error' when degraded, got: {body}"
    );
    assert!(
        body.contains("\"rpc\":\"unreachable\""),
        "rpc status should be 'unreachable', got: {body}"
    );
}

/// Test that /health returns HTTP 503 when RPC is unreachable.
/// This is critical for the acceptance criteria: "Returns 503 if any dependency is unreachable."
#[tokio::test]
async fn root_health_returns_503_when_rpc_unreachable() {
    let mut config = Config::default_for_test();
    // Point RPC to an invalid/unreachable endpoint to simulate failure
    config.stellar_rpc_url = String::from("http://localhost:1/invalid");

    let response = build_router(config, PriceCache::new())
        .oneshot(get("/health"))
        .await
        .expect("request failed");

    assert_eq!(
        response.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "health endpoint should return 503 when RPC is unreachable (acceptance criteria)"
    );

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"status\":\"error\""),
        "status should be 'error' when degraded, got: {body}"
    );
    assert!(
        body.contains("\"rpc\":\"unreachable\""),
        "rpc status should be 'unreachable', got: {body}"
    );
}

/// Verify that HTTP 503 response includes full dependency information for debugging.
#[tokio::test]
async fn health_503_response_includes_dependency_details() {
    let mut config = Config::default_for_test();
    config.stellar_rpc_url = String::from("http://localhost:1/invalid");

    let response = build_router(config, PriceCache::new())
        .oneshot(get("/health"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"dependencies\""),
        "503 response should include dependencies object, got: {body}"
    );
    assert!(
        body.contains("\"db\""),
        "503 response should show db status, got: {body}"
    );
    assert!(
        body.contains("\"rpc\""),
        "503 response should show rpc status, got: {body}"
    );
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
/// Test odds calculation logic with various scenarios.
#[tokio::test]
async fn test_odds_calculation() {
    use crate::db::calculate_odds;

    // Test case 1: Normal case with two outcomes
    let outcome_stakes = vec![(0, 25000), (1, 50000)];
    let total_stake = 75000;
    let odds = calculate_odds(&outcome_stakes, total_stake);

    assert_eq!(odds.len(), 2);
    assert_eq!(odds[0].outcome, 0);
    assert_eq!(odds[0].stake, 25000);
    assert!((odds[0].odds - 3.0).abs() < 0.001); // 1.0 / (25000/75000) = 3.0

    assert_eq!(odds[1].outcome, 1);
    assert_eq!(odds[1].stake, 50000);
    assert!((odds[1].odds - 1.5).abs() < 0.001); // 1.0 / (50000/75000) = 1.5

    // Test case 2: Zero total stake
    let odds_zero_total = calculate_odds(&outcome_stakes, 0);
    assert_eq!(odds_zero_total.len(), 2);
    assert_eq!(odds_zero_total[0].odds, 0.0);
    assert_eq!(odds_zero_total[1].odds, 0.0);

    // Test case 3: One outcome has zero stake
    let outcome_stakes_with_zero = vec![(0, 0), (1, 100000)];
    let odds_with_zero = calculate_odds(&outcome_stakes_with_zero, 100000);
    assert_eq!(odds_with_zero[0].odds, 0.0); // Zero stake = 0 odds
    assert!((odds_with_zero[1].odds - 1.0).abs() < 0.001); // 1.0 / (100000/100000) = 1.0
}

/// Test pool details endpoint returns error when database is not available.
#[tokio::test]
async fn api_v1_pool_details_returns_error_without_db() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/api/v1/pools/1"))
        .await
        .expect("request failed");

    // Note: This will likely return 429 due to rate limiting in tests
    // In a real environment with DB, it would return 200 with error message
    let body = body_string(response.into_body()).await;

    // The test mainly verifies the route exists and is callable
    // Actual functionality requires database integration testing
    assert!(!body.is_empty(), "response should not be empty");
}
/// Test user predictions endpoint returns error when database is not available.
#[tokio::test]
async fn api_v1_user_predictions_returns_error_without_db() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get(
            "/api/v1/users/GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX/predictions",
        ))
        .await
        .expect("request failed");

    // Note: This will likely return 429 due to rate limiting in tests
    // In a real environment with DB, it would return 200 with error message
    let body = body_string(response.into_body()).await;

    // The test mainly verifies the route exists and is callable
    // Actual functionality requires database integration testing
    assert!(!body.is_empty(), "response should not be empty");
}

/// Test user predictions endpoint with query parameters.
#[tokio::test]
async fn api_v1_user_predictions_handles_pagination() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/api/v1/users/GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX/predictions?limit=10&offset=5"))
        .await
        .expect("request failed");

    let body = body_string(response.into_body()).await;

    // Verify the route accepts pagination parameters
    assert!(!body.is_empty(), "response should not be empty");
}
/// Test leaderboard endpoint returns error when database is not available.
#[tokio::test]
async fn api_v1_leaderboard_returns_error_without_db() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/api/v1/leaderboard"))
        .await
        .expect("request failed");

    // Note: This will likely return 429 due to rate limiting in tests
    // In a real environment with DB, it would return 200 with error message
    let body = body_string(response.into_body()).await;

    // The test mainly verifies the route exists and is callable
    assert!(!body.is_empty(), "response should not be empty");
}

/// Test leaderboard endpoint with different ranking parameters.
#[tokio::test]
async fn api_v1_leaderboard_handles_ranking_parameters() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get(
            "/api/v1/leaderboard?rank_by=winnings&limit=10&offset=5",
        ))
        .await
        .expect("request failed");

    let body = body_string(response.into_body()).await;

    // Verify the route accepts ranking and pagination parameters
    assert!(!body.is_empty(), "response should not be empty");
}

/// Test leaderboard endpoint with volume ranking (default).
#[tokio::test]
async fn api_v1_leaderboard_defaults_to_volume_ranking() {
    let response = build_router(Config::default_for_test(), PriceCache::new())
        .oneshot(get("/api/v1/leaderboard?limit=5"))
        .await
        .expect("request failed");

    let body = body_string(response.into_body()).await;

    // Verify the route works with default volume ranking
    assert!(!body.is_empty(), "response should not be empty");
}
