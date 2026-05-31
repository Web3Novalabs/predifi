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
pub mod worker;
pub mod ws;

use axum::{extract::State, middleware::{from_fn_with_state, Next}, response::IntoResponse, routing::get, Json, Router};
use config::Config;
use http::HeaderValue;
use metrics::{Metrics, SharedMetrics};
use request_logger::LoggingLayer;
use serde_json::json;
use sentry::integrations::panic::register_panic_handler;
use sentry_tracing::layer as sentry_tracing_layer;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, Duration as TokioDuration};
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

/// Allowed frontend origins for CORS.
const ALLOWED_ORIGINS: &[&str] = &[
    "http://localhost:3000",
    "http://localhost:5173",
    "https://predifi.app",
];

/// Build the CORS middleware layer.
pub fn build_cors() -> CorsLayer {
    let origins: Vec<HeaderValue> = ALLOWED_ORIGINS
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

/// Health-check handler.
async fn health(State(state): State<routes::v1::AppState>) -> axum::response::Response {
    use axum::http::StatusCode;
    use std::time::Duration;

    let mut all_healthy = true;
    let mut db_status = "ok";

    if let Some(db) = &state.db {
        if sqlx::query("SELECT 1").execute(db).await.is_err() {
            db_status = "unreachable";
            all_healthy = false;
        }
    } else {
        db_status = "not_configured";
    }

    let mut rpc_status = "ok";
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(state.config.rpc_health_timeout_secs))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Try RPC health check with retry logic
    let mut rpc_attempts = 0;
    let max_attempts = state.config.rpc_health_retry_count as usize;
    let mut last_error = String::new();
    
    while rpc_attempts < max_attempts {
        rpc_attempts += 1;
        
        let rpc_req = client
            .post(&state.config.stellar_rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getHealth"
            }))
            .send()
            .await;

        match rpc_req {
            Ok(res) if res.status().is_success() => {
                // Success - break out of retry loop
                break;
            }
            Ok(res) => {
                last_error = format!("HTTP {} response", res.status());
            }
            Err(e) => {
                last_error = e.to_string();
            }
        }
        
        // Exponential backoff: 2^(attempt-1) seconds, capped at 5 seconds
        if rpc_attempts < max_attempts {
            let backoff_duration = std::cmp::min(2u64.pow((rpc_attempts - 1) as u32), 5);
            sleep(TokioDuration::from_secs(backoff_duration)).await;
        }
    }
    
    if rpc_attempts >= max_attempts {
        rpc_status = "unreachable";
        all_healthy = false;
    }

    let mut redis_status = "ok";
    if !state.redis.is_available() {
        redis_status = "not_configured";
        all_healthy = false;
    } else if !state.redis.ping().await {
        redis_status = "unreachable";
        all_healthy = false;
    }

    let mut price_cache_status = "ok";
    if state.cache.snapshot().is_empty() {
        price_cache_status = "not_ready";
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
            "rpc": if rpc_status == "unreachable" { Some(last_error.clone()) } else { None },
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
    state.metrics.update_memory_metrics();
    match state.metrics.gather_text() {
        Ok(body) => (
            axum::http::StatusCode::OK,
            [(http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
            body,
        ),
        Err(error) => {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                [(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                format!("failed to gather metrics: {error}"),
            )
        }
    }
}

async fn metrics_middleware<B>(
    State(metrics): State<SharedMetrics>,
    request: axum::http::Request<B>,
    next: Next<B>,
) -> impl IntoResponse {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let response = next.run(request).await;
    let status = response.status().as_u16().to_string();

    metrics
        .http_requests_total
        .with_label_values(&[&method, &path, &status])
        .inc();

    response
}

/// Build the Axum router with CORS, logging, and rate limiting middleware.
pub fn build_router(config: Config, cache: price_cache::PriceCache, redis: redis_cache::RedisCache, event_bus: ws::EventBus) -> Router {
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

    let prometheus_metrics = Arc::new(
        Metrics::new().unwrap_or_else(|error| {
            eprintln!("failed to initialize Prometheus metrics: {error}");
            std::process::exit(1);
        }),
    );

    let state = routes::v1::AppState {
        config: config.clone(),
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
        .nest("/api", routes::router(config, cache, redis, None, prometheus_metrics.clone(), event_bus))
        .merge(openapi::swagger_router())
        .layer(from_fn_with_state(metrics.clone(), metrics_middleware))
        .layer(GovernorLayer {
            config: governor_conf,
        })
        .layer(build_cors())
        .layer(LoggingLayer)
}

/// Build the Axum router with a live database pool.
pub fn build_router_with_db(
    config: Config,
    cache: price_cache::PriceCache,
    redis: redis_cache::RedisCache,
    pool: sqlx::PgPool,
    event_bus: ws::EventBus,
) -> Router {
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

    let prometheus_metrics = Arc::new(
        Metrics::new().unwrap_or_else(|error| {
            eprintln!("failed to initialize Prometheus metrics: {error}");
            std::process::exit(1);
        }),
    );

    let state = routes::v1::AppState {
        config: config.clone(),
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
        .nest("/api", routes::router_with_db(config, cache, redis, pool, prometheus_metrics.clone(), event_bus))
        .merge(openapi::swagger_router())
        .layer(from_fn_with_state(metrics.clone(), metrics_middleware))
        .layer(GovernorLayer {
            config: governor_conf,
        })
        .layer(build_cors())
        .layer(LoggingLayer)
}

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

    let pool = db::create_pool(&config).unwrap_or_else(|error| {
        error!(error = %error, "failed to initialize PostgreSQL pool");
        std::process::exit(1);
    });

    let cache = price_cache::PriceCache::new();
    price_cache::spawn_fetcher(cache.clone());

    let event_bus = ws::EventBus::new();

    // Spawn the on-chain event listener to keep pool and prediction indexes in sync.
    worker::stellar_listener::spawn(config.stellar_rpc_url.clone(), pool.clone(), event_bus.clone());

    // Initialize Redis cache
    let redis = redis_cache::RedisCache::new(&config.redis_url).await;
    if redis.is_available() {
        info!("Redis cache initialized and available");
    } else {
        warn!("Redis cache unavailable - running without caching");
    }

    let app = build_router_with_db(config.clone(), cache, redis, pool, event_bus);

    let bind_addr = config.bind_address();

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|error| {
            error!(address = %bind_addr, error = %error, "failed to bind TCP listener");
            std::process::exit(1);
        });

    info!(address = %bind_addr, "backend server listening");

    if let Err(error) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    {
        error!(error = %error, "server error");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod db_integration_tests;
#[cfg(test)]
mod tests;
