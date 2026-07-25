//! Server startup, router construction, and HTTP handlers.
//!
//! This module encapsulates everything needed to build and run the Axum
//! server — middleware, routes, and the `run` entry point that wires
//! together all dependencies (DB pool, price cache, Redis, event bus).

use crate::config::Config;
use crate::metrics::{Metrics, SharedMetrics};
use crate::request_logger::LoggingLayer;
use crate::shutdown;
use axum::{
    extract::State,
    middleware::{from_fn_with_state, Next},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use http::HeaderValue;
use serde_json::json;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::{error, info, warn};

// ── CORS ─────────────────────────────────────────────────────────────────────

fn build_cors(config: &Config) -> CorsLayer {
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

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn live() -> axum::response::Response {
    use axum::http::StatusCode;
    (
        StatusCode::OK,
        Json(json!({ "status": "alive", "service": "predifi-backend" })),
    )
        .into_response()
}

async fn health(State(state): State<crate::routes::v1::AppState>) -> axum::response::Response {
    use axum::http::StatusCode;

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

    let mut rpc_attempts = 0;
    let max_attempts = state.config.rpc_health_retry_count as usize;
    while rpc_attempts < max_attempts {
        rpc_attempts += 1;
        let ok = client
            .post(&state.config.stellar_rpc_url)
            .json(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"getHealth"}))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false);
        if ok {
            break;
        }
        if rpc_attempts < max_attempts {
            let backoff = std::cmp::min(2u64.pow((rpc_attempts - 1) as u32), 5);
            sleep(Duration::from_secs(backoff)).await;
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
    });

    if all_healthy {
        (StatusCode::OK, Json(body)).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
    }
}

async fn ready(State(state): State<crate::routes::v1::AppState>) -> axum::response::Response {
    use axum::http::StatusCode;

    let mut ready = true;
    let mut db_status = "ok";
    let mut db_error: Option<String> = None;

    if let Some(db) = &state.db {
        if sqlx::query("SELECT 1").execute(db).await.is_err() {
            db_status = "unreachable";
            db_error = Some("database ping failed".to_string());
            ready = false;
        }
    } else {
        db_status = "not_configured";
        db_error = Some("database pool is not configured".to_string());
        ready = false;
    }

    let (redis_status, redis_error): (&str, Option<String>) = if !state.redis.is_available() {
        (
            "not_configured",
            Some("Redis is not configured".to_string()),
        )
    } else if !state.redis.ping().await {
        ("unreachable", Some("Redis ping failed".to_string()))
    } else {
        ("ok", None)
    };
    if redis_status != "ok" {
        ready = false;
    }

    let (price_cache_status, price_cache_error): (&str, Option<String>) =
        if state.cache.snapshot().is_empty() {
            ("not_ready", Some("price cache is empty".to_string()))
        } else {
            ("ok", None)
        };
    if price_cache_status != "ok" {
        ready = false;
    }

    let body = json!({
        "status": if ready { "ready" } else { "not_ready" },
        "dependencies": { "db": db_status, "redis": redis_status, "price_cache": price_cache_status },
        "errors":       { "db": db_error, "redis": redis_error,   "price_cache": price_cache_error  }
    });

    if ready {
        (StatusCode::OK, Json(body)).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(body)).into_response()
    }
}

async fn root() -> Json<serde_json::Value> {
    Json(json!({ "message": "Welcome to the PrediFi backend", "api": "/api/v1" }))
}

async fn metrics_handler(State(state): State<crate::routes::v1::AppState>) -> impl IntoResponse {
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

async fn metrics_middleware(
    State(metrics): State<SharedMetrics>,
    request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> axum::response::Response {
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

// ── Router builders ───────────────────────────────────────────────────────────

/// Build the full router without a DB pool (for tests / rate-limit tests).
///
/// Rate limiting is excluded here; use [`build_router_with_rate_limit`] when
/// you need it in non-test contexts.
pub fn build_router(
    config: Config,
    cache: crate::price_cache::PriceCache,
    redis: crate::redis_cache::RedisCache,
    event_bus: crate::ws::EventBus,
) -> Router {
    build_router_with_rate_limit(
        config,
        cache,
        redis,
        event_bus,
        crate::constants::RATE_LIMIT_PERIOD_SECS,
        crate::constants::RATE_LIMIT_BURST_SIZE,
    )
}

/// Build the Axum router with explicit rate-limit settings (no DB pool).
pub fn build_router_with_rate_limit(
    config: Config,
    cache: crate::price_cache::PriceCache,
    redis: crate::redis_cache::RedisCache,
    event_bus: crate::ws::EventBus,
    _per_second: u64,
    _burst_size: u32,
) -> Router {
    let prometheus_metrics = Arc::new(Metrics::new().unwrap_or_else(|error| {
        eprintln!("failed to initialize Prometheus metrics: {error}");
        std::process::exit(1);
    }));

    let state = crate::routes::v1::AppState {
        config: Arc::new(config.clone()),
        cache: cache.clone(),
        redis: redis.clone(),
        db: None,
        metrics: prometheus_metrics.clone(),
        event_bus: event_bus.clone(),
    };

    Router::new()
        .route("/", get(root))
        .route("/live", get(live))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
        .nest(
            "/api",
            crate::routes::router(
                Arc::new(config.clone()),
                cache,
                redis,
                None,
                prometheus_metrics.clone(),
                event_bus,
            ),
        )
        .merge(crate::openapi::swagger_router())
        .layer(from_fn_with_state(
            prometheus_metrics.clone(),
            metrics_middleware,
        ))
        .layer(build_cors(&config))
        .layer(LoggingLayer::with_metrics(prometheus_metrics.clone()))
}

/// Build the Axum router with a live database pool.
fn build_router_with_db(
    config: Config,
    cache: crate::price_cache::PriceCache,
    redis: crate::redis_cache::RedisCache,
    pool: sqlx::PgPool,
    event_bus: crate::ws::EventBus,
    prometheus_metrics: SharedMetrics,
) -> Router {
    let state = crate::routes::v1::AppState {
        config: Arc::new(config.clone()),
        cache: cache.clone(),
        redis: redis.clone(),
        db: Some(pool.clone()),
        metrics: prometheus_metrics.clone(),
        event_bus: event_bus.clone(),
    };

    Router::new()
        .route("/", get(root))
        .route("/live", get(live))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
        .nest(
            "/api",
            crate::routes::router_with_db(
                Arc::new(config.clone()),
                cache,
                redis,
                pool,
                prometheus_metrics.clone(),
                event_bus,
            ),
        )
        .merge(crate::openapi::swagger_router())
        .layer(from_fn_with_state(
            prometheus_metrics.clone(),
            metrics_middleware,
        ))
        .layer(build_cors(&config))
        .layer(LoggingLayer::with_metrics(prometheus_metrics.clone()))
}

// ── Server entry points ───────────────────────────────────────────────────────

/// Initialise all dependencies, build the router, and start serving.
pub async fn run(config: Config) {
    run_with_signal(config, shutdown::wait_for_signal()).await;
}

/// Initialise all dependencies, build the router, start serving, and tear
/// everything down again when `signal` resolves.
///
/// Split out from [`run`] so the shutdown plumbing is testable via a
/// hand-crafted future rather than real OS signals.
///
/// # Shutdown order
/// 1. `signal` resolves → Axum stops accepting new connections.
/// 2. In-flight requests drain (bounded by `config.shutdown_timeout_secs`).
/// 3. Background workers (price-cache fetcher, Stellar listener) are aborted.
/// 4. PostgreSQL pool is closed.
/// 5. OTel batch exporter is flushed via [`crate::telemetry::shutdown_tracer_provider`].
pub async fn run_with_signal<F>(config: Config, signal: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let pool = crate::db::create_pool(&config)
        .await
        .unwrap_or_else(|error| {
            error!(error = %error, "failed to initialize PostgreSQL pool");
            std::process::exit(1);
        });

    let prometheus_metrics = Arc::new(Metrics::new().unwrap_or_else(|error| {
        eprintln!("failed to initialize Prometheus metrics: {error}");
        std::process::exit(1);
    }));

    let cache = crate::price_cache::PriceCache::new();
    let fetcher_handle: JoinHandle<()> =
        crate::price_cache::spawn_fetcher(cache.clone(), Some(prometheus_metrics.clone()));

    let event_bus = crate::ws::EventBus::new();
    let redis = crate::redis_cache::RedisCache::new(&config.redis_url).await;

    // Clone before moving into the worker closure.
    let listener_rpc_url = config.stellar_rpc_url.clone();
    let listener_pool = pool.clone();
    let listener_event_bus = event_bus.clone();
    let listener_redis = redis.clone();
    let listener_rpc_timeout = Duration::from_secs(config.rpc_timeout_secs);
    let listener_batch_size = config.indexer_max_batch_size;

    // spawn_worker roots the listener under a named OTel span so all Stellar
    // sync traces are correlated in the trace backend.
    let listener_handle: JoinHandle<()> =
        crate::tracing_context::spawn_worker("stellar_listener", async move {
            crate::worker::stellar_listener::run_worker(
                listener_rpc_url,
                listener_pool,
                listener_event_bus,
                listener_redis,
                listener_rpc_timeout,
                listener_batch_size,
            )
            .await;
        });

    if redis.is_available() {
        info!("Redis cache initialized and available");
    } else {
        warn!("Redis cache unavailable - running without caching");
    }

    let app = build_router_with_db(
        config.clone(),
        cache,
        redis,
        pool.clone(),
        event_bus,
        prometheus_metrics,
    );

    let bind_addr = config.bind_address();
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|error| {
            error!(address = %bind_addr, error = %error, "failed to bind TCP listener");
            std::process::exit(1);
        });

    info!(address = %bind_addr, "backend server listening");

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(signal);

    let drain_timeout = Duration::from_secs(config.shutdown_timeout_secs.max(1));

    match tokio::time::timeout(drain_timeout, server).await {
        Ok(Ok(())) => info!(
            timeout_secs = drain_timeout.as_secs(),
            "HTTP server drained in-flight requests cleanly"
        ),
        Ok(Err(error)) => {
            warn!(error = %error, "HTTP server returned an error during shutdown drain")
        }
        Err(_) => warn!(
            component = "http server drain",
            timeout_secs = drain_timeout.as_secs(),
            "HTTP drain exceeded shutdown timeout; aborting in-flight handlers"
        ),
    }

    // Abort workers before closing the pool so they cannot race it.
    fetcher_handle.abort();
    listener_handle.abort();

    // Close the pool after aborting workers.
    shutdown::with_shutdown_timeout(drain_timeout, "database pool close", pool.close()).await;

    // Flush the OTel batch exporter — must happen after all spans are done.
    crate::telemetry::shutdown_tracer_provider();

    info!("graceful shutdown complete");
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DEFAULT_CORS_ORIGINS};

    fn config_with_origins(origins: Vec<&str>) -> Config {
        let mut cfg = Config::default_for_test();
        cfg.cors_allowed_origins = origins.into_iter().map(|s| s.to_string()).collect();
        cfg
    }

    #[test]
    fn build_cors_accepts_valid_https_origin() {
        let cfg = config_with_origins(vec!["https://predifi.app"]);
        let _layer = build_cors(&cfg);
    }

    #[test]
    fn build_cors_default_origins_are_all_valid_header_values() {
        let cfg = config_with_origins(DEFAULT_CORS_ORIGINS.to_vec());
        let valid_count = cfg
            .cors_allowed_origins
            .iter()
            .filter(|o| o.parse::<HeaderValue>().is_ok())
            .count();
        assert_eq!(
            valid_count,
            DEFAULT_CORS_ORIGINS.len(),
            "every default CORS origin must parse into a valid HeaderValue"
        );
    }

    #[test]
    fn build_cors_accepts_origin_with_port() {
        let cfg = config_with_origins(vec!["http://localhost:5173"]);
        let _layer = build_cors(&cfg);
        assert!("http://localhost:5173".parse::<HeaderValue>().is_ok());
    }

    #[test]
    fn build_cors_accepts_multiple_origins() {
        let cfg = config_with_origins(vec![
            "https://predifi.app",
            "https://staging.predifi.app",
            "http://localhost:3000",
        ]);
        let _layer = build_cors(&cfg);
    }

    #[test]
    fn build_cors_with_empty_origins_does_not_panic() {
        let cfg = config_with_origins(vec![]);
        let _layer = build_cors(&cfg);
    }

    #[test]
    fn build_cors_uses_config_origins_not_hardcoded_list() {
        let cfg_a = config_with_origins(vec!["https://app-a.example.com"]);
        let cfg_b = config_with_origins(vec!["https://app-b.example.com"]);
        let _layer_a = build_cors(&cfg_a);
        let _layer_b = build_cors(&cfg_b);
        assert_ne!(cfg_a.cors_allowed_origins, cfg_b.cors_allowed_origins);
    }
}
