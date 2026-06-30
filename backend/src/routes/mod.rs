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

        let response = app
            .oneshot(get("/v1/fees"))
            .await
            .expect("request failed");

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_string(response.into_body()).await;
        assert!(
            body.contains("\"treasury_fee_bps\":250")
                && body.contains("\"referral_fee_bps\":5000"),
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
}
