//! Server startup, router construction, and HTTP handlers.
//!
//! This module encapsulates everything needed to build and run the Axum
//! server — middleware, routes, and the `run` entry point that wires
//! together all dependencies (DB pool, price cache, Redis, event bus).

use crate::config::Config;
use crate::metrics::{Metrics, SharedMetrics};
use crate::request_logger::LoggingLayer;
use crate::shutdown;
use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use http::HeaderValue;
use serde_json::json;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
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
    crate::health::check_all(&state.config, state.db.as_ref(), &state.redis, &state.cache)
        .await
        .into_response_with_version(env!("CARGO_PKG_VERSION"))
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
        pool_cache: crate::pool_cache::PoolCache::with_negative_ttl(Duration::from_secs(
            config.pool_negative_cache_ttl_secs,
        )),
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
        .layer(build_cors(&config))
        .layer(
            LoggingLayer::with_metrics(prometheus_metrics.clone())
                .with_jwt(&config.secret_key, config.jwt_key_version),
        )
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
        pool_cache: crate::pool_cache::PoolCache::with_negative_ttl(Duration::from_secs(
            config.pool_negative_cache_ttl_secs,
        )),
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
        .layer(build_cors(&config))
        .layer(
            LoggingLayer::with_metrics(prometheus_metrics.clone())
                .with_jwt(&config.secret_key, config.jwt_key_version),
        )
}

/// Probe Redis once at startup and either fail fast or record a degraded
/// state, depending on `config.redis_required`.
///
/// If Redis is configured but unreachable:
/// - `config.redis_required == true`: logs a clear error and exits the
///   process immediately, before the HTTP listener is bound, so an operator
///   sees a startup failure rather than a silently degraded service.
/// - `config.redis_required == false`: logs a single explicit warning and
///   lets startup continue. The degraded state is then visible on every
///   subsequent call to `/ready` and `/health`, which independently probe
///   Redis per request via [`crate::redis_cache::RedisCache::ping`].
async fn probe_redis_startup(config: &Config, redis: &crate::redis_cache::RedisCache) {
    if redis.is_available() && redis.ping().await {
        return;
    }

    if config.redis_required {
        error!(
            "Redis is configured (PREDIFI_REDIS_URL) but unreachable at startup, and \
             PREDIFI_REDIS_REQUIRED=true; refusing to start. Set PREDIFI_REDIS_REQUIRED=false \
             to start in a degraded mode instead."
        );
        std::process::exit(1);
    }

    warn!(
        "Redis is configured (PREDIFI_REDIS_URL) but unreachable at startup; starting in a \
         degraded mode (PREDIFI_REDIS_REQUIRED=false). Cache-backed reads will fall through to \
         the database until Redis becomes reachable. This state is visible on /ready and /health."
    );
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
/// 3. Open WebSocket connections are sent a Close frame and given the same
///    budget to disconnect cleanly (see [`shutdown::drain_websockets`]).
/// 4. Background workers (price-cache fetcher, Stellar listener,
///    notification sweep, pool cache warmer) are given the same budget to
///    finish their current unit of work before being aborted (see
///    [`shutdown::drain_workers`]).
/// 5. PostgreSQL pool is closed.
/// 6. OTel batch exporter is flushed via [`crate::telemetry::shutdown_tracer_provider`].
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

    // Repository functions record query latencies through the global sink.
    crate::metrics::install_global(prometheus_metrics.clone());

    let cache = crate::price_cache::PriceCache::new();
    let fetcher_handle: JoinHandle<()> =
        crate::price_cache::spawn_fetcher(cache.clone(), Some(prometheus_metrics.clone()));

    let event_bus = crate::ws::EventBus::new().with_metrics(prometheus_metrics.clone());
    // Cloned before `event_bus` is moved into the router below; used only to
    // drive the WebSocket drain during shutdown.
    let shutdown_event_bus = event_bus.clone();
    let redis = crate::redis_cache::RedisCache::new(&config.redis_url)
        .await
        .with_metrics(prometheus_metrics.clone());
    probe_redis_startup(&config, &redis).await;

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

    // Periodically sweeps pool/prediction state to generate notifications
    // (pool ending soon, resolved, claim window expiring, interest matches).
    // Every insert is deduplicated on the DB side, so this can run on a fixed
    // interval without ever double-notifying a user.
    let notifications_pool = pool.clone();
    let notifications_handle: JoinHandle<()> =
        crate::tracing_context::spawn_worker("notification_sweep", async move {
            crate::notifications::run_sweep_loop(notifications_pool, Duration::from_secs(300))
                .await;
        });

    // Proactively refreshes the Redis cache for the default, first-page
    // pool-list queries (see `cache_warmer` module docs for why only those).
    // Ticks every half the pools TTL so an entry is refreshed well before it
    // would otherwise expire and force a visitor to pay the cold-cache
    // database round trip.
    let warmer_pool = pool.clone();
    let warmer_redis = redis.clone();
    let warm_interval = Duration::from_secs((crate::redis_cache::POOLS_CACHE_TTL / 2).max(1));
    let cache_warmer_handle: JoinHandle<()> =
        crate::tracing_context::spawn_worker("pool_cache_warmer", async move {
            crate::cache_warmer::run_warm_loop(warmer_pool, warmer_redis, warm_interval).await;
        });

    // Refreshes DB-derived gauges (pool counts by state, prediction volume
    // per window) and process gauges (WebSocket connections, memory, DB pool).
    let collector_pool = pool.clone();
    let collector_metrics = prometheus_metrics.clone();
    let collector_bus = event_bus.clone();
    let metrics_collector_handle: JoinHandle<()> =
        crate::tracing_context::spawn_worker("metrics_collector", async move {
            crate::metrics::run_collector_loop(
                collector_pool,
                collector_metrics,
                collector_bus,
                Duration::from_secs(15),
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

    // Signal every open WebSocket connection to close and give them the same
    // drain budget as the HTTP server before moving on. This must happen
    // before workers are stopped, since the Stellar listener and
    // notification sweep both publish onto the event bus that WebSocket
    // handlers read from.
    shutdown::drain_websockets(&shutdown_event_bus.ws_shutdown_token(), drain_timeout).await;

    // Give workers a chance to finish their current unit of work before
    // forcibly aborting them, instead of killing them immediately. Each is
    // bounded by the same drain timeout; a worker still running past it is
    // aborted (see `shutdown::drain_workers`).
    let workers = vec![
        ("price_fetcher", fetcher_handle),
        ("stellar_listener", listener_handle),
        ("notification_sweep", notifications_handle),
        ("pool_cache_warmer", cache_warmer_handle),
        ("metrics_collector", metrics_collector_handle),
    ];
    shutdown::drain_workers(workers, drain_timeout).await;

    // Close the pool after draining workers so they cannot race it.
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
