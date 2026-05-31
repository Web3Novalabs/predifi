//! # predifi-backend
//!
//! A minimal Axum HTTP server with CORS and request-logging middleware.

pub mod config;
pub mod db;
pub mod metrics;
pub mod openapi;
pub mod price_cache;
pub mod redis_cache;
pub mod referrals;
pub mod request_logger;
pub mod response;
pub mod routes;
pub mod server;
pub mod worker;
pub mod ws;

pub use server::build_router;

use config::Config;
use sentry::integrations::panic::register_panic_handler;
use sentry_tracing::layer as sentry_tracing_layer;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config::from_env().unwrap_or_else(|error| {
        eprintln!("failed to load configuration: {error}");
        std::process::exit(1);
    });

    if let Some(dsn) = config.sentry_dsn.as_ref() {
        let _guard = sentry::init((
            dsn.as_str(),
            sentry::ClientOptions {
                release: Some(env!("CARGO_PKG_VERSION").into()),
                ..Default::default()
            },
        ));
        register_panic_handler();

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt()
                    .with_env_filter(EnvFilter::new(config.log_level.clone()))
                    .with_target(false)
                    .compact(),
            )
            .with(sentry_tracing_layer())
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new(config.log_level.clone()))
            .with_target(false)
            .compact()
            .init();
    }

    info!("starting predifi-backend server");

    server::run(config).await;
}

#[cfg(test)]
mod db_integration_tests;
#[cfg(test)]
mod redis_integration_tests;
#[cfg(test)]
mod tests;
