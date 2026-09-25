use axum::{body::Body, http::{HeaderValue, Request}, routing::get, Router};
use predifi_backend::{build_cors, config::Config};
use tower::ServiceExt;

fn config_with_origins(origins: Vec<&str>) -> Config {
    let mut cfg = Config::default_for_test();
    cfg.cors_allowed_origins = origins.into_iter().map(str::to_owned).collect();
    cfg
}

#[tokio::test]
async fn build_cors_preserves_valid_origin_list() {
    let cfg = config_with_origins(vec!["https://app.example.com", "https://admin.example.com"]);
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(build_cors(&cfg));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("Origin", "https://app.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.headers().get("access-control-allow-origin").unwrap(),
        HeaderValue::from_static("https://app.example.com")
    );
}

#[tokio::test]
async fn build_cors_drops_unparseable_origin_without_panicking() {
    let cfg = config_with_origins(vec![
        "https://good.example.com",
        "bad origin with spaces",
    ]);
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(build_cors(&cfg));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("Origin", "https://good.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.headers().get("access-control-allow-origin").unwrap(),
        HeaderValue::from_static("https://good.example.com")
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header("Origin", "bad origin with spaces")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.headers().get("access-control-allow-origin").is_none());
}
