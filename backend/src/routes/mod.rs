use crate::config::Config;
use crate::price_cache::PriceCache;
use axum::Router;

pub mod v1;

/// Build the API router tree.
pub fn router(config: Config, cache: PriceCache, pool: Option<sqlx::PgPool>) -> Router {
    Router::new().nest("/v1", v1::router(config, cache, pool))
}
