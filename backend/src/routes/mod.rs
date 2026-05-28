use crate::config::Config;
use crate::price_cache::PriceCache;
use crate::redis_cache::RedisCache;
use crate::ws::EventBus;
use axum::Router;
use sqlx::PgPool;

pub mod v1;

pub fn router(
    config: Config,
    cache: PriceCache,
    redis: RedisCache,
    pool: Option<sqlx::PgPool>,
    metrics: crate::metrics::SharedMetrics,
    event_bus: EventBus,
) -> Router {
    Router::new().nest("/v1", v1::router(config, cache, redis, pool, metrics, event_bus))
}

pub fn router_with_db(
    config: Config,
    cache: PriceCache,
    redis: RedisCache,
    db: PgPool,
    metrics: crate::metrics::SharedMetrics,
    event_bus: EventBus,
) -> Router {
    Router::new().nest("/v1", v1::router(config, cache, redis, Some(db), metrics, event_bus))
}
