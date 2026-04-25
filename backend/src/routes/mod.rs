use crate::config::Config;
use axum::Router;
use sqlx::PgPool;

pub mod v1;

/// Build the API router tree (no DB — used in tests).
pub fn router(config: Config) -> Router {
    Router::new().nest("/v1", v1::router(config))
}

/// Build the API router tree with a live database pool.
pub fn router_with_db(config: Config, db: PgPool) -> Router {
    Router::new().nest("/v1", v1::router_with_db(config, db))
}
