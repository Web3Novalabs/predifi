use crate::config::Config;
use crate::price_cache::PriceCache;
use crate::redis_cache::RedisCache;
use axum::Router;
use sqlx::PgPool;

pub mod v1;

/// Build the API router tree.
pub fn router(config: Config, cache: PriceCache, redis: RedisCache, pool: Option<sqlx::PgPool>) -> Router {
    Router::new().nest("/v1", v1::router(config, cache, redis, pool))
}

/// Build the API router tree with a live database pool.
pub fn router_with_db(config: Config, cache: PriceCache, redis: RedisCache, db: PgPool) -> Router {
    Router::new().nest("/v1", v1::router(config, cache, redis, Some(db)))
}
