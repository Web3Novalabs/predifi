use axum::{Router, body::Body, http::Request, routing::get};
use tower::ServiceExt;
use uuid::Uuid;

use backend::middleware::request_id_middleware;

async fn test_handler() -> &'static str {
    "Hello, World!"
}

#[tokio::test]
async fn test_request_id_middleware() {
    // Create a simple app with our middleware
    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(request_id_middleware());

    // Test 1: Request without X-Request-ID header (should generate one)
    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.clone().oneshot(request).await.unwrap();

    // Check that response has X-Request-ID header
    let request_id = response.headers().get("X-Request-ID");
    assert!(
        request_id.is_some(),
        "Response should have X-Request-ID header"
    );

    let request_id_value = request_id.unwrap().to_str().unwrap();
    assert!(
        !request_id_value.is_empty(),
        "Request ID should not be empty"
    );

    // Verify it's a valid UUID
    assert!(
        Uuid::parse_str(request_id_value).is_ok(),
        "Request ID should be a valid UUID"
    );

    // Test 2: Request with existing X-Request-ID header (should preserve it)
    let existing_request_id = "test-request-id-12345";
    let request = Request::builder()
        .uri("/test")
        .header("X-Request-ID", existing_request_id)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Check that response has the same X-Request-ID header
    let request_id = response.headers().get("X-Request-ID");
    assert!(
        request_id.is_some(),
        "Response should have X-Request-ID header"
    );

    let request_id_value = request_id.unwrap().to_str().unwrap();
    assert_eq!(
        request_id_value, existing_request_id,
        "Request ID should be preserved"
    );
}

#[tokio::test]
async fn test_request_id_middleware_with_empty_header() {
    // Create a simple app with our middleware
    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(request_id_middleware());

    // Test: Request with empty X-Request-ID header (should generate new one)
    let request = Request::builder()
        .uri("/test")
        .header("X-Request-ID", "")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Check that response has X-Request-ID header
    let request_id = response.headers().get("X-Request-ID");
    assert!(
        request_id.is_some(),
        "Response should have X-Request-ID header"
    );

    let request_id_value = request_id.unwrap().to_str().unwrap();
    assert!(
        !request_id_value.is_empty(),
        "Request ID should not be empty"
    );

    // Verify it's a valid UUID (should generate new one since original was empty)
    assert!(
        Uuid::parse_str(request_id_value).is_ok(),
        "Request ID should be a valid UUID"
    );
}
