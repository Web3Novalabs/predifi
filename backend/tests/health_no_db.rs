use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use predifi_backend::{
    build_router,
    config::Config,
    price_cache::PriceCache,
    redis_cache::RedisCache,
    ws,
};
use tower::ServiceExt;

fn get(path: &str) -> Request<axum::body::Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .body(axum::body::Body::empty())
        .expect("failed to build request")
}

async fn body_string(body: axum::body::Body) -> String {
    let bytes = body
        .collect()
        .await
        .expect("failed to collect body")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("body is not valid utf-8")
}

#[tokio::test]
async fn health_reports_db_not_configured_without_pool_and_still_responds() {
    let router = build_router(
        Config::default_for_test(),
        PriceCache::new(),
        RedisCache::disabled(),
        ws::EventBus::new(),
    );

    let response = router
        .oneshot(get("/health/components"))
        .await
        .expect("request failed");

    let status = response.status();
    assert!(
        status == StatusCode::OK || status == StatusCode::SERVICE_UNAVAILABLE,
        "health endpoint should respond even when DB is absent; got status {status}"
    );

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"db\":\"not_configured\""),
        "body should report db as not_configured, got: {body}"
    );
}

/// `/health` is a liveness probe: it answers 200 without touching any
/// dependency, so a missing database must not turn it into a failure.
#[tokio::test]
async fn health_is_liveness_and_stays_200_without_a_pool() {
    let router = build_router(
        Config::default_for_test(),
        PriceCache::new(),
        RedisCache::disabled(),
        ws::EventBus::new(),
    );

    let response = router
        .oneshot(get("/health"))
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"status\":\"alive\""),
        "body should report the process as alive, got: {body}"
    );
    assert!(
        !body.contains("\"dependencies\""),
        "liveness must not probe dependencies, got: {body}"
    );
}

/// `/health/ready` is the readiness probe: without a database pool the
/// instance cannot serve traffic, so it must answer 503.
#[tokio::test]
async fn health_ready_reports_not_ready_without_a_pool() {
    let router = build_router(
        Config::default_for_test(),
        PriceCache::new(),
        RedisCache::disabled(),
        ws::EventBus::new(),
    );

    let response = router
        .oneshot(get("/health/ready"))
        .await
        .expect("request failed");

    assert_eq!(
        response.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "readiness must fail while the database is missing"
    );

    let body = body_string(response.into_body()).await;
    assert!(
        body.contains("\"ready\":false") && body.contains("\"db\":\"not_configured\""),
        "body should report the instance as not ready with the db status, got: {body}"
    );
}
