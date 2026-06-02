//! # predifi-backend
//!
//! A minimal Axum HTTP server with CORS and request-logging middleware.

pub mod config;
pub mod constants;
pub mod db;
pub mod jwt;
pub mod metrics;
pub mod session;
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

use crate::config::Config;
use sentry_tracing::layer as sentry_tracing_layer;
use tracing::info;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config::from_env().unwrap_or_else(|error| {
        eprintln!("failed to load configuration: {error}");
        std::process::exit(1);
    });

    let filter = EnvFilter::new(config.log_level.clone());
    let use_json = config.app_env == "production";

    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

    let registry = tracing_subscriber::registry().with(filter);

    if use_json {
        let registry = registry.with(fmt_layer.json());
        if let Some(dsn) = config.sentry_dsn.as_ref() {
            let _guard = sentry::init((
                dsn.as_str(),
                sentry::ClientOptions {
                    release: Some(env!("CARGO_PKG_VERSION").into()),
                    ..Default::default()
                },
            ));
            registry.with(sentry_tracing_layer()).init();
        } else {
            registry.init();
        }
    } else {
        let registry = registry.with(fmt_layer.compact());
        if let Some(dsn) = config.sentry_dsn.as_ref() {
            let _guard = sentry::init((
                dsn.as_str(),
                sentry::ClientOptions {
                    release: Some(env!("CARGO_PKG_VERSION").into()),
                    ..Default::default()
                },
            ));
            registry.with(sentry_tracing_layer()).init();
        } else {
            registry.init();
        }
    }

    info!("starting predifi-backend server");

    server::run(config).await;
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod db_integration_tests;
#[cfg(test)]
mod redis_integration_tests;
#[cfg(test)]
mod tests;
