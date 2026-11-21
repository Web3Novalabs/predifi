use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use predifi_backend::{build_router, config::Config, price_cache::PriceCache, redis_cache::RedisCache, ws};
use tower::ServiceExt;

fn get(path: &str) -> Request<axum::body::Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .body(axum::body::Body::empty())
        .expect("failed to build request")
}

fn get_with_bearer(path: &str, token: &str) -> Request<axum::body::Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .header("Authorization", format!("Bearer {token}"))
        .body(axum::body::Body::empty())
        .expect("failed to build request")
}

#[tokio::test]
async fn metrics_returns_prometheus_content_type_and_body() {
    let router = build_router(
        Config::default_for_test(),
        PriceCache::new(),
        RedisCache::disabled(),
        ws::EventBus::new(),
    );

    let response = router
        .oneshot(get("/metrics"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get(http::header::CONTENT_TYPE)
        .expect("content-type header must be present")
        .to_str()
        .expect("content-type header must be valid utf-8");
    assert_eq!(content_type, "text/plain; version=0.0.4");

    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("failed to collect body")
        .to_bytes();
    assert!(!bytes.is_empty(), "metrics body must not be empty");
}

// When PREDIFI_METRICS_TOKEN is set, the endpoint must reject requests that
// omit the bearer token or supply a wrong one.

#[tokio::test]
async fn metrics_returns_401_when_token_configured_and_no_auth_header() {
    let mut config = Config::default_for_test();
    config.metrics_token = Some(String::from("super-secret-token"));

    let router = build_router(config, PriceCache::new(), RedisCache::disabled(), ws::EventBus::new());

    let response = router
        .oneshot(get("/metrics"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn metrics_returns_401_when_token_configured_and_wrong_token() {
    let mut config = Config::default_for_test();
    config.metrics_token = Some(String::from("super-secret-token"));

    let router = build_router(config, PriceCache::new(), RedisCache::disabled(), ws::EventBus::new());

    let response = router
        .oneshot(get_with_bearer("/metrics", "wrong-token"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn metrics_returns_200_when_token_configured_and_correct_token() {
    let mut config = Config::default_for_test();
    config.metrics_token = Some(String::from("super-secret-token"));

    let router = build_router(config, PriceCache::new(), RedisCache::disabled(), ws::EventBus::new());

    let response = router
        .oneshot(get_with_bearer("/metrics", "super-secret-token"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
}
