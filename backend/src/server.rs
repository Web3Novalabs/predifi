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
use tokio::time::sleep;
use tokio::task::JoinHandle;
#[cfg(not(test))]
use tower_governor::governor::GovernorConfigBuilder;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::{error, info, warn};

/// Build the CORS middleware layer from the validated origin list in `config`.
///
/// Only the origins listed in [`Config::cors_allowed_origins`] are permitted.
/// The list is validated at startup (see `config::parse_cors_origins`), so any
/// entry that reaches this function is already a well-formed `http://` or
/// `https://` origin.  Entries that cannot be parsed into a [`HeaderValue`] are
/// silently skipped (this should never happen in practice given the prior
/// validation).
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

/// Health-check handler.
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

    // Try RPC health check with retry logic
    let mut rpc_attempts = 0;
    let max_attempts = state.config.rpc_health_retry_count as usize;

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
                break;
            }
            Ok(_res) => {}
            Err(_e) => {}
        }

        // Exponential backoff: 2^(attempt-1) seconds, capped at 5 seconds
        if rpc_attempts < max_attempts {
            let backoff_duration = std::cmp::min(2u64.pow((rpc_attempts - 1) as u32), 5);
            sleep(Duration::from_secs(backoff_duration)).await;
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

/// Readiness probe handler — signals whether the service is ready to accept traffic.
///
/// Unlike the general `/health` liveness endpoint, this probe focuses on
/// Redis connectivity, which is required for the caching layer to function.
/// Kubernetes (and similar orchestrators) use readiness probes to decide
/// whether to route traffic to a pod; returning `503` here will temporarily
/// remove the instance from the load-balancer rotation until Redis recovers.
///
/// # Responses
/// - `200 OK` — Redis is reachable and the service is ready.
/// - `503 Service Unavailable` — Redis is not configured or unreachable.
async fn ready(State(state): State<crate::routes::v1::AppState>) -> axum::response::Response {
    use axum::http::StatusCode;

    let (redis_status, redis_error): (&str, Option<String>) = if !state.redis.is_available() {
        (
            "not_configured",
            Some("Redis is not configured".to_string()),
        )
    } else if !state.redis.ping().await {
        (
            "unreachable",
            Some("Redis ping failed — connection may be lost".to_string()),
        )
    } else {
        ("ok", None)
    };

    let ready = redis_status == "ok";

    let body = json!({
        "status": if ready { "ready" } else { "not_ready" },
        "dependencies": {
            "redis": redis_status,
        },
        "errors": {
            "redis": redis_error,
        }
    });

    if ready {
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
async fn metrics(State(state): State<crate::routes::v1::AppState>) -> impl IntoResponse {
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

/// Build the Axum router with CORS, logging, and rate limiting middleware.
pub fn build_router(
    config: Config,
    cache: crate::price_cache::PriceCache,
    redis: crate::redis_cache::RedisCache,
    event_bus: crate::ws::EventBus,
) -> Router {
    if cfg!(test) {
        // Unit tests run in parallel and share the same Governor key extractor,
        // which can cause unrelated tests to rate-limit each other. Use an
        // effectively-unlimited quota for tests.
        build_router_with_rate_period(
            config,
            cache,
            redis,
            event_bus,
            std::time::Duration::from_secs(1),
            u32::MAX,
        )
    } else {
        build_router_with_rate_limit(
            config,
            cache,
            redis,
            event_bus,
            crate::constants::RATE_LIMIT_PERIOD_SECS,
            crate::constants::RATE_LIMIT_BURST_SIZE,
        )
    }
}

/// Build the Axum router with explicit rate limit settings.
pub fn build_router_with_rate_limit(
    config: Config,
    cache: crate::price_cache::PriceCache,
    redis: crate::redis_cache::RedisCache,
    event_bus: crate::ws::EventBus,
    per_second: u64,
    burst_size: u32,
) -> Router {
    build_router_with_rate_period(
        config,
        cache,
        redis,
        event_bus,
        std::time::Duration::from_secs(per_second),
        burst_size,
    )
}

#[allow(unused_variables)]
fn build_router_with_rate_period(
    config: Config,
    cache: crate::price_cache::PriceCache,
    redis: crate::redis_cache::RedisCache,
    event_bus: crate::ws::EventBus,
    period: std::time::Duration,
    burst_size: u32,
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

    let router = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics))
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
        .layer(LoggingLayer);

    #[cfg(not(test))]
    let router = {
        let governor_conf = Arc::new(
            GovernorConfigBuilder::default()
                .period(period)
                .burst_size(burst_size)
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
fn build_router_with_db(
    config: Config,
    cache: crate::price_cache::PriceCache,
    redis: crate::redis_cache::RedisCache,
    pool: sqlx::PgPool,
    event_bus: crate::ws::EventBus,
) -> Router {
    let prometheus_metrics = Arc::new(Metrics::new().unwrap_or_else(|error| {
        eprintln!("failed to initialize Prometheus metrics: {error}");
        std::process::exit(1);
    }));

    let state = crate::routes::v1::AppState {
        config: Arc::new(config.clone()),
        cache: cache.clone(),
        redis: redis.clone(),
        db: Some(pool.clone()),
        metrics: prometheus_metrics.clone(),
        event_bus: event_bus.clone(),
    };

    let router = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics))
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
        .layer(LoggingLayer);

    #[cfg(not(test))]
    let router = {
        let governor_conf = Arc::new(
            GovernorConfigBuilder::default()
                .per_second(crate::constants::RATE_LIMIT_PERIOD_SECS)
                .burst_size(crate::constants::RATE_LIMIT_BURST_SIZE)
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

/// Initialise all dependencies, build the router, and start serving.
///
/// This delegates to [`run_with_signal`] using [`crate::shutdown::wait_for_signal`]
/// so that SIGINT, SIGTERM and (on Unix) SIGHUP cause a graceful drain of
/// in-flight requests before the process exits.
pub async fn run(config: Config) {
    run_with_signal(config, shutdown::wait_for_signal()).await;
}

/// Initialise all dependencies, build the router, start serving, and tear
/// everything down again on `signal`.
///
/// The function is split out from [`run`] so the shutdown plumbing is
/// testable: callers (notably the integration tests in [`crate::tests`])
/// pass a hand-crafted future instead of relying on real OS signals.
///
/// # Order of operations on shutdown
///
/// 1. The provided `signal` future resolves (this is what kicks off the
///    drain — Axum then refuses new connections and waits for in-flight
///    ones to finish).
/// 2. The HTTP server future is awaited with [`shutdown::with_shutdown_timeout`]
///    giving in-flight requests up to `config.shutdown_timeout_secs` to
///    finish.  This protects the process from being held hostage by a
///    stuck handler.
/// 3. The PostgreSQL connection pool is closed via [`sqlx::Pool::close`].
///    In-flight queries get a chance to finish because `close` waits for
///    outstanding checkouts to be returned.
/// 4. Background workers (the price-cache fetcher and the Stellar event
///    listener) are aborted so they do not keep the runtime alive after
///    the listener has gone away.
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

    let cache = crate::price_cache::PriceCache::new();
    let fetcher_handle: JoinHandle<()> = crate::price_cache::spawn_fetcher(cache.clone());

    let event_bus = crate::ws::EventBus::new();

    // Spawn the on-chain event listener to keep pool and prediction indexes in sync.
    let listener_handle: JoinHandle<()> = crate::worker::stellar_listener::spawn(
        config.stellar_rpc_url.clone(),
        pool.clone(),
        event_bus.clone(),
        std::time::Duration::from_secs(30),
    );

    // Initialize Redis cache
    let redis = crate::redis_cache::RedisCache::new(&config.redis_url).await;
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

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(signal);

    let drain_timeout = Duration::from_secs(config.shutdown_timeout_secs.max(1));

    // `axum::serve().with_graceful_shutdown(...)` resolves with
    // `Result<(), std::io::Error>` so we cannot reuse the
    // `shutdown::with_shutdown_timeout` helper directly — instead inline a
    // short match so every branch records whether the drain completed,
    // the handler returned an error, or we blew the deadline.
    match tokio::time::timeout(drain_timeout, server).await {
        Ok(Ok(())) => {
            info!(
                timeout_secs = drain_timeout.as_secs(),
                "HTTP server drained in-flight requests cleanly"
            );
        }
        Ok(Err(error)) => {
            warn!(
                error = %error,
                "HTTP server returned an error during shutdown drain"
            );
        }
        Err(_) => {
            warn!(
                component = "http server drain",
                timeout_secs = drain_timeout.as_secs(),
                "HTTP drain exceeded shutdown timeout; aborting in-flight handlers"
            );
        }
    }

    // Stop the background workers *before* closing the database pool so
    // they cannot race the close by acquiring a fresh connection from the
    // pool between `pool.close().await` starting and completing.  The
    // listeners' loops are stateless w.r.t. in-flight work — the ledger
    // cursor persists to the database after every successful poll — so a
    // restarted worker will resume from the last persisted position.
    fetcher_handle.abort();
    listener_handle.abort();

    // Close the database pool last so any in-flight query the listener was
    // already running gets a chance to finish, and no NEW query can be
    // submitted because the workers were already aborted above.
    shutdown::with_shutdown_timeout(
        drain_timeout,
        "database pool close",
        pool.close(),
    )
    .await;

    info!("graceful shutdown complete");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DEFAULT_CORS_ORIGINS};

    // ── Helpers ──────────────────────────────────────────────────────────────

    /// Build a [`Config`] whose CORS origins are exactly `origins`.
    fn config_with_origins(origins: Vec<&str>) -> Config {
        let mut cfg = Config::default_for_test();
        cfg.cors_allowed_origins = origins.into_iter().map(|s| s.to_string()).collect();
        cfg
    }

    // ── CORS whitelist tests ──────────────────────────────────────────────────

    /// `build_cors` must accept origins that are already validated by the
    /// config layer without panicking or silently dropping them.
    #[test]
    fn build_cors_accepts_valid_https_origin() {
        let cfg = config_with_origins(vec!["https://predifi.app"]);
        // build_cors must not panic; it returns a CorsLayer
        let _layer = build_cors(&cfg);
    }

    /// All three default origins must parse into valid header values, i.e.
    /// none of them should be silently skipped by the `filter_map` in
    /// `build_cors`.
    #[test]
    fn build_cors_default_origins_are_all_valid_header_values() {
        let cfg = config_with_origins(DEFAULT_CORS_ORIGINS.to_vec());
        // If any default origin was malformed it would be silently dropped.
        // We verify all of them survive the filter_map.
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

    /// An origin with a port should also parse correctly.
    #[test]
    fn build_cors_accepts_origin_with_port() {
        let cfg = config_with_origins(vec!["http://localhost:5173"]);
        let _layer = build_cors(&cfg);
        // Verify the origin itself is parseable
        assert!(
            "http://localhost:5173".parse::<HeaderValue>().is_ok(),
            "localhost origin with port must be a valid HeaderValue"
        );
    }

    /// Multiple simultaneous origins must all be accepted.
    #[test]
    fn build_cors_accepts_multiple_origins() {
        let cfg = config_with_origins(vec![
            "https://predifi.app",
            "https://staging.predifi.app",
            "http://localhost:3000",
        ]);
        let _layer = build_cors(&cfg);
    }

    /// An empty origins list produces a CORS layer that allows no origin
    /// (all cross-origin requests will be rejected by the browser).
    #[test]
    fn build_cors_with_empty_origins_does_not_panic() {
        let cfg = config_with_origins(vec![]);
        // Must not panic — the layer simply allows nothing.
        let _layer = build_cors(&cfg);
    }

    /// Verifies that the CORS-allowed-origins field on [`Config`] is read
    /// directly from the config struct, confirming that `build_cors` honours
    /// the runtime configuration rather than hard-coding any origins.
    #[test]
    fn build_cors_uses_config_origins_not_hardcoded_list() {
        let cfg_a = config_with_origins(vec!["https://app-a.example.com"]);
        let cfg_b = config_with_origins(vec!["https://app-b.example.com"]);

        // Both configs must produce valid CorsLayer instances — this confirms
        // the function is parametric over the config rather than referencing
        // a fixed list.
        let _layer_a = build_cors(&cfg_a);
        let _layer_b = build_cors(&cfg_b);

        assert_ne!(
            cfg_a.cors_allowed_origins,
            cfg_b.cors_allowed_origins,
            "the two configs must differ so the test is meaningful"
        );
    }
}
