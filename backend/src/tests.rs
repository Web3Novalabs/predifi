use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt; // provides `.oneshot()`

// Pull in the router builder from main.rs.
use crate::build_router;

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
    let response = build_router()
        .oneshot(get("/"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
}

/// GET /health must return HTTP 200 with `{"status":"ok"}` in the body.
#[tokio::test]
async fn health_returns_200_with_ok_body() {
    let response = build_router()
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
    let response = build_router()
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
    let response = build_router()
        .oneshot(get("/api/v1"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
}

/// GET /nonexistent must return HTTP 404 (Axum's built-in fallback).
#[tokio::test]
async fn unknown_route_returns_404() {
    let response = build_router()
        .oneshot(get("/nonexistent"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Verify the middleware does not alter the status code of a 200 response.
#[tokio::test]
async fn middleware_does_not_alter_200_status() {
    let response = build_router()
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
    let response = build_router()
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
        let response = build_router()
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
    let response = build_router()
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
    let response = build_router()
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
