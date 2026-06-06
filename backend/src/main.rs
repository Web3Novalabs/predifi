//! # predifi-backend
//!
//! A minimal Axum HTTP server with CORS and request-logging middleware.

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
pub mod server;
pub mod session;
pub mod telemetry;
pub mod worker;
pub mod ws;

use crate::config::Config;
use crate::metrics::Metrics;
use crate::request_logger::LoggingLayer;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Json;
use axum::Router;
use http::header::HeaderValue;
use sentry_tracing::layer as sentry_tracing_layer;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration as TokioDuration;
use tokio::time::sleep;
#[cfg(not(test))]
use tower_governor::governor::GovernorConfigBuilder;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::info;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

/// Build the CORS middleware layer from the validated origin list in `config`.
///
/// Only the origins listed in [`Config::cors_allowed_origins`] are permitted.
/// The list is validated at startup (see `config::parse_cors_origins`), so any
/// entry that reaches this function is already a well-formed `http://` or
/// `https://` origin.  Entries that cannot be parsed into a [`HeaderValue`] are
/// silently skipped (this should never happen in practice given the prior
/// validation).
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

use axum::extract::State;

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

        // Exponential backoff: 2^(attempt) seconds, capped at 5 seconds
        if attempt < max_attempts - 1 {
            let backoff_duration = std::cmp::min(2u64.pow(attempt as u32), 5);
            sleep(TokioDuration::from_secs(backoff_duration)).await;
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

    /// Check Redis health and availability.
    async fn check_redis_health(redis: &redis_cache::RedisCache) -> (String, String) {
        if !redis.is_available() {
            return ("not_configured".to_string(), String::new());
        }
        if !redis.ping().await {
            return ("unreachable".to_string(), "Redis ping failed".to_string());
        }
        ("ok".to_string(), String::new())
    }

    /// Check price cache health.
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

// async fn metrics_middleware(
//     State(metrics): State<SharedMetrics>,
//     request: axum::http::Request<axum::body::Body>,
//     next: Next,
// ) -> axum::response::Response {
//     let method = request.method().to_string();
//     let path = request.uri().path().to_string();
//
//     let response = next.run(request).await;
//     let status = response.status().as_u16().to_string();
//
//     metrics
//         .http_requests_total
//         .with_label_values(&[&method, &path, &status])
//         .inc();
//
//     response
// }

/// Build the Axum router with CORS, logging, and rate limiting middleware.
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

    let router = Router::new()
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
        .layer(LoggingLayer);

    #[cfg(not(test))]
    let router = {
        let governor_conf = Arc::new(
            GovernorConfigBuilder::default()
                .per_second(5)
                .burst_size(50)
                .error_handler(|_| {
                    (
                        axum::http::StatusCode::TOO_MANY_REQUESTS,
                        "Too Many Requests",
                    )
                        .into_response()
                })
                .finish()
                .unwrap(),
        );
        router.layer(tower_governor::GovernorLayer {
            config: governor_conf,
        })
    };

    router
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

    #[cfg(not(test))]
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(5)
            .burst_size(50)
            .error_handler(|_| {
                (
                    axum::http::StatusCode::TOO_MANY_REQUESTS,
                    "Too Many Requests",
                )
                    .into_response()
            })
            .finish()
            .unwrap(),
    );

    let router = Router::new()
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
        .layer(LoggingLayer);

    #[cfg(not(test))]
    let router = router.layer(tower_governor::GovernorLayer {
        config: governor_conf,
    });

    router
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config::from_env().unwrap_or_else(|error| {
        eprintln!("failed to load configuration: {error}");
        std::process::exit(1);
    });

    let filter = EnvFilter::new(config.log_level.clone());
    let use_json = true;

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
            let registry = registry.with(sentry_tracing_layer());
            registry.init();
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
            let registry = registry.with(sentry_tracing_layer());
            registry.init();
        } else {
            registry.init();
        }
    }

    info!("starting predifi-backend server");

    server::run(config).await;
}

#[cfg(test)]
mod db_integration_tests;
#[cfg(test)]
mod redis_integration_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
