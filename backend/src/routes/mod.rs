use crate::config::Config;
use axum::Router;

pub mod v1;

/// Build the API router tree.
///
/// Versioned sub-routers make it easier to grow the API without restructuring
/// the top-level application router.
pub fn router(config: Config) -> Router {
    Router::new().nest("/v1", v1::router(config))
}
