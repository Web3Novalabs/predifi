use crate::config::Config;
use crate::price_cache::PriceCache;
use crate::redis_cache::RedisCache;
use crate::ws::EventBus;
use axum::Router;
use sqlx::PgPool;
use std::sync::Arc;

pub mod v1;

/// Build the versioned API router without a database pool.
///
/// All routes that require a database will return an error response at
/// runtime. Useful for unit tests and health-check-only deployments.
pub fn router(
    config: Arc<Config>,
    cache: PriceCache,
    redis: RedisCache,
    pool: Option<sqlx::PgPool>,
    metrics: crate::metrics::SharedMetrics,
    event_bus: EventBus,
) -> Router {
    Router::new().nest(
        "/v1",
        v1::router(config, cache, redis, pool, metrics, event_bus),
    )
}

/// Build the versioned API router with a live PostgreSQL connection pool.
///
/// Wraps the pool in `Some` and delegates to [`router`].
pub fn router_with_db(
    config: Arc<Config>,
    cache: PriceCache,
    redis: RedisCache,
    db: PgPool,
    metrics: crate::metrics::SharedMetrics,
    event_bus: EventBus,
) -> Router {
    Router::new().nest(
        "/v1",
        v1::router(config, cache, redis, Some(db), metrics, event_bus),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    use crate::config::Config;
    use crate::metrics::Metrics;
    use crate::price_cache::PriceCache;
    use crate::redis_cache::RedisCache;
    use crate::ws::EventBus;

    use super::{router, router_with_db};

    fn test_metrics() -> crate::metrics::SharedMetrics {
        Arc::new(Metrics::new().expect("metrics must initialize in tests"))
    }

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

    fn build_router_without_db() -> axum::Router {
        router(
            Arc::new(Config::default_for_test()),
            PriceCache::new(),
            RedisCache::disabled(),
            None,
            test_metrics(),
            EventBus::new(),
        )
    }

    fn build_router_with_lazy_db() -> axum::Router {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(1))
            .connect_lazy("postgres://localhost:5432/predifi_routes_test")
            .expect("lazy pool");
        router_with_db(
            Arc::new(Config::default_for_test()),
            PriceCache::new(),
            RedisCache::disabled(),
            pool,
            test_metrics(),
            EventBus::new(),
        )
    }

    /// `router` must nest the v1 API under `/v1`.
    #[tokio::test]
    async fn router_nests_v1_index() {
        let response = build_router_without_db()
            .oneshot(get("/v1"))
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_string(response.into_body()).await;
        assert!(
            body.contains("\"name\":\"predifi-backend\"") && body.contains("\"version\":\"v1\""),
            "body should describe the v1 API, got: {body}"
        );
    }

    /// Routes that require a database must fail gracefully when no pool is passed.
    #[tokio::test]
    async fn router_without_db_returns_database_unavailable_for_pools() {
        let response = build_router_without_db()
            .oneshot(get("/v1/pools"))
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = body_string(response.into_body()).await;
        assert!(
            body.contains("database not available"),
            "body should mention missing database, got: {body}"
        );
    }

    /// `router` must still expose stateless routes such as `/v1/fees`.
    #[tokio::test]
    async fn router_without_db_returns_fees() {
        let mut config = Config::default_for_test();
        config.treasury_fee_bps = 250;
        config.referral_fee_bps = 5000;

        let app = router(
            Arc::new(config),
            PriceCache::new(),
            RedisCache::disabled(),
            None,
            test_metrics(),
            EventBus::new(),
        );

        let response = app.oneshot(get("/v1/fees")).await.expect("request failed");

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_string(response.into_body()).await;
        assert!(
            body.contains("\"treasury_fee_bps\":250") && body.contains("\"referral_fee_bps\":5000"),
            "body should contain configured fee values, got: {body}"
        );
    }

    /// Unknown paths under the nested router must return 404.
    #[tokio::test]
    async fn router_unknown_route_returns_404() {
        let response = build_router_without_db()
            .oneshot(get("/v1/no-such-route"))
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// `router_with_db` must wire the pool through so DB-backed routes attempt queries.
    #[tokio::test]
    async fn router_with_db_attempts_database_queries() {
        let response = build_router_with_lazy_db()
            .oneshot(get("/v1/pools"))
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = body_string(response.into_body()).await;
        assert!(
            !body.contains("database not available"),
            "router_with_db should pass a pool to handlers, got: {body}"
        );
        assert!(
            body.contains("\"error\"") || body.contains("error"),
            "unreachable database should surface a query error, got: {body}"
        );
    }

    /// `router_with_db` must nest routes under `/v1` like `router`.
    #[tokio::test]
    async fn router_with_db_nests_v1_index() {
        let response = build_router_with_lazy_db()
            .oneshot(get("/v1"))
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_string(response.into_body()).await;
        assert!(
            body.contains("\"version\":\"v1\""),
            "router_with_db should nest under /v1, got: {body}"
        );
    }

    // ── Market predictions: no-DB guard ──────────────────────────────────────

    /// Without a DB the market predictions endpoint must return a structured
    /// DATABASE_UNAVAILABLE error (not a panic or 500 without an envelope).
    #[tokio::test]
    async fn market_predictions_without_db_returns_database_unavailable() {
        let response = build_router_without_db()
            .oneshot(get("/v1/markets/1/predictions"))
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = body_string(response.into_body()).await;
        assert!(
            body.contains("database not available") || body.contains("DATABASE_UNAVAILABLE"),
            "should surface DATABASE_UNAVAILABLE, got: {body}"
        );
    }

    /// A non-numeric market ID must return 422 (Axum path extraction failure).
    #[tokio::test]
    async fn market_predictions_bad_market_id_returns_400() {
        let response = build_router_without_db()
            .oneshot(get("/v1/markets/not-a-number/predictions"))
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// `?limit` values outside the 1-100 range are clamped, not rejected.
    /// With no DB the response is DATABASE_UNAVAILABLE – the important thing is
    /// that no 500 / panic occurs from an unclamped limit.
    #[tokio::test]
    async fn market_predictions_large_limit_is_clamped() {
        let response = build_router_without_db()
            .oneshot(get("/v1/markets/1/predictions?limit=99999"))
            .await
            .expect("request failed");

        // DATABASE_UNAVAILABLE path, but must not be a 500 from bad arithmetic.
        assert_ne!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// With a lazy (unreachable) DB the handler must attempt a query and return
    /// a DB error response, not the database-unavailable sentinel (which would
    /// mean the pool was never wired through).
    #[tokio::test]
    async fn market_predictions_with_lazy_db_attempts_query() {
        let response = build_router_with_lazy_db()
            .oneshot(get("/v1/markets/1/predictions"))
            .await
            .expect("request failed");

        let body = body_string(response.into_body()).await;
        assert!(
            !body.contains("database not available"),
            "router_with_db should wire the pool; got: {body}"
        );
        // Should either be a DB error or a 404 (market not found), never a panic.
        assert!(
            body.contains("\"error\""),
            "should return an error envelope; got: {body}"
        );
    }

    fn post_json(path: &str, body: &str) -> Request<axum::body::Body> {
        Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json")
            .body(axum::body::Body::from(body.to_owned()))
            .expect("failed to build request")
    }

    // ── POST /v1/waitlist (#2101) ────────────────────────────────────────────

    #[tokio::test]
    async fn waitlist_invalid_email_returns_400() {
        for email in ["", "nope", "a@b", "a b@c.com"] {
            let body = format!(r#"{{"name":"Ada","email":"{email}"}}"#);
            let response = build_router_without_db()
                .oneshot(post_json("/v1/waitlist", &body))
                .await
                .expect("request failed");
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "email {email:?}");
        }
    }

    #[tokio::test]
    async fn waitlist_blank_name_returns_400() {
        let response = build_router_without_db()
            .oneshot(post_json(
                "/v1/waitlist",
                r#"{"name":"   ","email":"ada@example.com"}"#,
            ))
            .await
            .expect("request failed");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn waitlist_missing_field_is_rejected() {
        let response = build_router_without_db()
            .oneshot(post_json("/v1/waitlist", r#"{"email":"ada@example.com"}"#))
            .await
            .expect("request failed");
        assert!(response.status().is_client_error());
    }

    /// Validation runs before the DB check, and a valid request with no DB must
    /// not claim success (503, never 200).
    #[tokio::test]
    async fn waitlist_valid_request_without_db_returns_503() {
        let response = build_router_without_db()
            .oneshot(post_json(
                "/v1/waitlist",
                r#"{"name":"Ada","email":"Ada@Example.com"}"#,
            ))
            .await
            .expect("request failed");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = body_string(response.into_body()).await;
        assert!(body.contains("database not available"), "got: {body}");
    }

    /// With a (lazy, unreachable) pool wired in, a valid request must reach the
    /// insert rather than short-circuit.
    #[tokio::test]
    async fn waitlist_valid_request_with_lazy_db_attempts_insert() {
        let response = build_router_with_lazy_db()
            .oneshot(post_json(
                "/v1/waitlist",
                r#"{"name":"Ada","email":"ada@example.com"}"#,
            ))
            .await
            .expect("request failed");
        let body = body_string(response.into_body()).await;
        assert!(!body.contains("database not available"), "got: {body}");
    }

    // ── PATCH /v1/pools/:id/tags: admin-only (#2068) ─────────────────────────

    /// Retagging a pool requires the admin key. Without `x-admin-api-key` the
    /// request must be rejected even when the body names the (public) creator.
    #[tokio::test]
    async fn pool_tags_update_without_admin_key_returns_401() {
        let request = Request::builder()
            .method("PATCH")
            .uri("/v1/pools/1/tags")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(
                r#"{"creator":"GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H","tags":["hidden"]}"#,
            ))
            .expect("failed to build request");

        let response = build_router_without_db()
            .oneshot(request)
            .await
            .expect("request failed");

        let status = response.status();
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "PATCH tags without admin key must be 401, got {status}"
        );
    }

    /// A correct `x-admin-api-key` passes authentication for any `creator`
    /// value (the field is ignored). The router has no DB, so the handler
    /// reaches the database check and returns 503 instead of 401.
    #[tokio::test]
    async fn pool_tags_update_with_admin_key_passes_auth_for_any_creator() {
        let key = "test-admin-key-2068";
        std::env::set_var("PREDIFI_ADMIN_API_KEY", key);

        for creator in [
            "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        ] {
            let request = Request::builder()
                .method("PATCH")
                .uri("/v1/pools/1/tags")
                .header("content-type", "application/json")
                .header("x-admin-api-key", key)
                .body(axum::body::Body::from(
                    serde_json::json!({ "creator": creator, "tags": ["featured"] }).to_string(),
                ))
                .expect("failed to build request");

            let response = build_router_without_db()
                .oneshot(request)
                .await
                .expect("request failed");

            let status = response.status();
            assert_eq!(
                status,
                StatusCode::SERVICE_UNAVAILABLE,
                "PATCH tags with admin key must pass auth (503 without DB), got {status}"
            );
        }
        std::env::remove_var("PREDIFI_ADMIN_API_KEY");
    }
}
