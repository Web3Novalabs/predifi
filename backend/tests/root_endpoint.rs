use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use predifi_backend::{build_router, config::Config, price_cache::PriceCache, redis_cache::RedisCache, ws};
use serde_json::Value;
use tower::ServiceExt;

fn get(path: &str) -> Request<axum::body::Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .body(axum::body::Body::empty())
        .expect("failed to build request")
}

#[tokio::test]
async fn root_returns_documented_shape() {
    let router = build_router(
        Config::default_for_test(),
        PriceCache::new(),
        RedisCache::disabled(),
        ws::EventBus::new(),
    );

    let response = router.oneshot(get("/")).await.expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("failed to collect body")
        .to_bytes();
    let body: Value = serde_json::from_slice(&bytes).expect("body must be valid json");

    let object = body.as_object().expect("body must be a json object");
    assert_eq!(object.len(), 2, "body must have exactly two keys, got: {body}");

    assert_eq!(
        object.get("message").and_then(Value::as_str),
        Some("Welcome to the PrediFi backend")
    );
    assert_eq!(
        object.get("api").and_then(Value::as_str),
        Some("/api/v1")
    );
}
