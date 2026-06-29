//! # predifi-backend (library)
//!
//! Library crate shared by the `predifi-backend` server binary and the
//! `predifi-seed` database seeding binary.  All modules, router builders, and
//! handlers live here so both binaries (and the test suite) share a single
//! source of truth.

pub mod config;
pub mod constants;
pub mod db;
pub mod errors;
pub mod jwt;
pub mod metrics;
pub mod openapi;
pub mod price_cache;
pub mod redis_cache;
pub mod referrals;
pub mod request_logger;
pub mod response;
pub mod routes;
pub mod seed;
pub mod server;
pub mod session;
pub mod shutdown;
pub mod telemetry;
pub mod tracing_context;
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
use std::time::Duration as TokioDuration;
use tokio::time::sleep;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::info;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

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

/// Check database health with a simple query.
async fn check_db_health(db: &Option<sqlx::PgPool>) -> (String, String) {
    if let Some(pool) = db {
        match sqlx::query("SELECT 1").execute(pool).await {
            Ok(_) => ("ok".to_string(), String::new()),
            Err(e) => ("unreachable".to_string(), e.to_string()),
        }
    } else {
        ("not_configured".to_string(), String::new())
    }
}

/// Check RPC health with retry logic and exponential backoff.
async fn check_rpc_health(rpc_url: &str, timeout_secs: u64, retry_count: u8) -> (String, String) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let max_attempts = retry_count as usize;
    let mut last_error = String::new();

    for attempt in 0..max_attempts {
        let rpc_req = client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getHealth"
            }))
            .send()
            .await;

        match rpc_req {
            Ok(res) if res.status().is_success() => {
                return ("ok".to_string(), String::new());
            }
            Ok(res) => {
                last_error = format!("HTTP {} response", res.status());
            }
            Err(e) => {
                last_error = e.to_string();
            }
        }

        if attempt < max_attempts - 1 {
            let backoff = std::cmp::min(2u64.pow(attempt as u32), 5);
            sleep(TokioDuration::from_secs(backoff)).await;
        }
    }

    ("unreachable".to_string(), last_error)
}

/// Health-check handler.
async fn health(State(state): State<routes::v1::AppState>) -> axum::response::Response {
    use axum::http::StatusCode;

    let mut all_healthy = true;
    let (db_status, db_error) = check_db_health(&state.db).await;
    if db_status == "unreachable" {
        all_healthy = false;
    }

    let (rpc_status, _rpc_error) = check_rpc_health(
        &state.config.stellar_rpc_url,
        state.config.rpc_health_timeout_secs,
        state.config.rpc_health_retry_count,
    )
    .await;
    if rpc_status == "unreachable" {
        all_healthy = false;
    }

    async fn check_redis_health(redis: &redis_cache::RedisCache) -> (String, String) {
        if !redis.is_available() {
            return ("not_configured".to_string(), String::new());
        }
        if !redis.ping().await {
            return ("unreachable".to_string(), "Redis ping failed".to_string());
        }
        ("ok".to_string(), String::new())
    }

    fn check_price_cache_health(cache: &price_cache::PriceCache) -> (String, String) {
        if cache.snapshot().is_empty() {
            return ("not_ready".to_string(), "price cache is empty".to_string());
        }
        ("ok".to_string(), String::new())
    }

    let (redis_status, redis_error) = check_redis_health(&state.redis).await;
    if redis_status == "unreachable" || redis_status == "not_configured" {
        all_healthy = false;
    }

    let (price_cache_status, _price_cache_error) = check_price_cache_health(&state.cache);
    if price_cache_status == "not_ready" {
        all_healthy = false;
    }

    let body = json!({
        "status": if all_healthy { "ok" } else { "error" },
        "service": "predifi-backend",
        "version": env!("CARGO_PKG_VERSION"),
        "dependencies": {
            "db": db_status,
            "rpc": rpc_status,
            "redis": redis_status,
            "price_cache": price_cache_status
        },
        "errors": {
            "db": if db_status == "unreachable" { Some(db_error.clone()) } else { None },
            "rpc": if rpc_status == "unreachable" { Some("rpc unreachable".to_string()) } else { None },
            "redis": if redis_status == "unreachable" { Some(redis_error.clone()) } else { None },
            "price_cache": if price_cache_status == "not_ready" { Some("price cache is empty".to_string()) } else { None }
        }
    });

    if all_healthy {
        (StatusCode::OK, Json(body)).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
    }
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
        .layer(LoggingLayer::with_metrics(prometheus_metrics.clone()))
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
        .layer(LoggingLayer::with_metrics(prometheus_metrics.clone()))
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

    if let Some(tracer) = otel_tracer {
        // Full OTel stack: EnvFilter + OTel layer + fmt layer.
        telemetry::init_tracing_subscriber(tracer, &log_level, true);
    } else {
        // No OTel — plain fmt subscriber.
        let filter = EnvFilter::new(&log_level);
        let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer.json())
            .init();
    }

    info!("starting predifi-backend server");

    server::run(config).await;
}

#[cfg(all(test, feature = "integration-tests"))]
mod db_integration_tests;
#[cfg(all(test, feature = "integration-tests"))]
mod redis_integration_tests;
#[cfg(all(test, feature = "integration-tests"))]
mod test_support;
#[cfg(test)]
mod mock_rpc_helpers;
#[cfg(test)]
mod tests;
