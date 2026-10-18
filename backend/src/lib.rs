//! # predifi-backend (library)
//!
//! Library crate shared by the `predifi-backend` server binary and the
//! `predifi-seed` database seeding binary.  All modules, router builders, and
//! handlers live here so both binaries (and the test suite) share a single
//! source of truth.

pub mod cache_warmer;
pub mod config;
pub mod constants;
pub mod db;
pub mod email;
pub mod errors;
pub mod health;
pub mod jwt;
pub mod jwt_security;
pub mod metrics;
pub mod migration_checks;
pub mod notifications;
pub mod openapi;
pub mod pool_cache;
pub mod price_cache;
pub mod profile;
pub mod rate_limit;
pub mod redis_cache;
pub mod referrals;
pub mod request_logger;
pub mod response;
pub mod routes;
pub mod seed;
pub mod server;
pub mod session;
pub mod shutdown;
pub mod tags;
pub mod telemetry;
pub mod tracing_context;
pub mod validated_types;
pub mod worker;
pub mod ws;

use crate::config::Config;
use crate::metrics::Metrics;
use crate::request_logger::LoggingLayer;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Json;
use axum::Router;
use http::HeaderValue;
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::info;

/// Build the CORS middleware layer from the validated origin list in `config`.
pub fn build_cors(config: &Config) -> CorsLayer {
    let origins: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| origin.parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            http::Method::GET,
            http::Method::POST,
            http::Method::PUT,
            http::Method::DELETE,
            http::Method::OPTIONS,
        ])
        .allow_headers([
            http::header::CONTENT_TYPE,
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
        ])
}

/// Health-check handler (see [`health`]).
async fn health(State(state): State<routes::v1::AppState>) -> axum::response::Response {
    health::check_all(&state.config, state.db.as_ref(), &state.redis, &state.cache)
        .await
        .into_response_with_version(env!("CARGO_PKG_VERSION"))
}

/// Root handler — returns a welcome message.
async fn root() -> Json<serde_json::Value> {
    Json(json!({
        "message": "Welcome to the PrediFi backend",
        "api": "/api/v1"
    }))
}

/// Metrics endpoint exposed to Prometheus.
async fn metrics(State(state): State<routes::v1::AppState>) -> impl IntoResponse {
    match state.metrics.gather_text() {
        Ok(body) => (
            axum::http::StatusCode::OK,
            [(http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
            body,
        ),
        Err(error) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            [(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            format!("failed to gather metrics: {error}"),
        ),
    }
}

/// Build the Axum router (no DB pool, no rate limiting — for tests / health-only deployments).
pub fn build_router(
    config: Config,
    cache: price_cache::PriceCache,
    redis: redis_cache::RedisCache,
    event_bus: ws::EventBus,
) -> Router {
    let prometheus_metrics = Arc::new(Metrics::new().unwrap_or_else(|error| {
        eprintln!("failed to initialize Prometheus metrics: {error}");
        std::process::exit(1);
    }));

    let state = routes::v1::AppState {
        config: Arc::new(config.clone()),
        cache: cache.clone(),
        pool_cache: crate::pool_cache::PoolCache::with_negative_ttl(
            std::time::Duration::from_secs(config.pool_negative_cache_ttl_secs),
        ),
        redis: redis.clone(),
        db: None,
        metrics: prometheus_metrics.clone(),
        event_bus: event_bus.clone(),
    };

    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .with_state(state)
        .nest(
            "/api",
            routes::router(
                Arc::new(config.clone()),
                cache,
                redis,
                None,
                prometheus_metrics.clone(),
                event_bus,
            ),
        )
        .merge(openapi::swagger_router())
        .layer(build_cors(&config))
        .layer(
            LoggingLayer::with_metrics(prometheus_metrics.clone())
                .with_jwt(&config.secret_key, config.jwt_key_version),
        )
}

/// Build the Axum router with a live database pool.
pub fn build_router_with_db(
    config: Config,
    cache: price_cache::PriceCache,
    redis: redis_cache::RedisCache,
    pool: sqlx::PgPool,
    event_bus: ws::EventBus,
) -> Router {
    let prometheus_metrics = Arc::new(Metrics::new().unwrap_or_else(|error| {
        eprintln!("failed to initialize Prometheus metrics: {error}");
        std::process::exit(1);
    }));

    let state = routes::v1::AppState {
        config: Arc::new(config.clone()),
        cache: cache.clone(),
        pool_cache: crate::pool_cache::PoolCache::with_negative_ttl(
            std::time::Duration::from_secs(config.pool_negative_cache_ttl_secs),
        ),
        redis: redis.clone(),
        db: Some(pool.clone()),
        metrics: prometheus_metrics.clone(),
        event_bus: event_bus.clone(),
    };

    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .with_state(state)
        .nest(
            "/api",
            routes::router_with_db(
                Arc::new(config.clone()),
                cache,
                redis,
                pool,
                prometheus_metrics.clone(),
                event_bus,
            ),
        )
        .merge(openapi::swagger_router())
        .layer(build_cors(&config))
        .layer(
            LoggingLayer::with_metrics(prometheus_metrics.clone())
                .with_jwt(&config.secret_key, config.jwt_key_version),
        )
}

/// Initialise tracing, build the server, and run it to completion.
pub async fn run_server(config: Config) {
    config.validate().unwrap_or_else(|error| {
        eprintln!("configuration validation failed: {error}");
        std::process::exit(1);
    });

    // Try to initialise the OTel tracer provider.
    // When TELEMETRY_ENABLED is not "true" (the default) this is a no-op and
    // we fall back to a plain fmt subscriber so the server works without a
    // collector configured.
    let otel_tracer = telemetry::init_telemetry_from_env();
    let log_level = config.log_level.clone();

    // JSON by default; PREDIFI_LOG_FORMAT=pretty for local development.
    let use_json = telemetry::LogFormat::from_env() == telemetry::LogFormat::Json;

    if let Some(tracer) = otel_tracer {
        // Full OTel stack: reloadable EnvFilter + OTel layer + fmt layer.
        telemetry::init_tracing_subscriber(tracer, &log_level, use_json);
    } else {
        // No OTel — reloadable plain subscriber so the admin endpoint can
        // still change the log level without a restart.
        telemetry::init_plain_subscriber(&log_level, use_json);
    }

    info!("starting predifi-backend server");

    server::run(config).await;
}

#[cfg(all(test, feature = "integration-tests"))]
mod db_integration_tests;
#[cfg(test)]
mod mock_rpc_helpers;
#[cfg(test)]
mod openapi_tests;
#[cfg(all(test, feature = "integration-tests"))]
mod redis_integration_tests;
#[cfg(all(test, feature = "integration-tests"))]
mod test_support;
#[cfg(test)]
mod tests;
