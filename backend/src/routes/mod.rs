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
