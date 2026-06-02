//! Server startup, router construction, and HTTP handlers.
//!
//! This module encapsulates everything needed to build and run the Axum
//! server — middleware, routes, and the `run` entry point that wires
//! together all dependencies (DB pool, price cache, Redis, event bus).

use crate::config::Config;
use crate::metrics::{Metrics, SharedMetrics};
use crate::request_logger::LoggingLayer;
use axum::{
    extract::State,
    middleware::{from_fn_with_state, Next},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use http::HeaderValue;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, Duration as TokioDuration};
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

/// Simple health-check handler returning 200 OK.
async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

/// Detailed health-check handler.
async fn health_detailed(
    State(state): State<crate::routes::v1::AppState>,
) -> axum::response::Response {
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
            Ok(_res) => {
                // RPC call failed with non-success status
            }
            Err(_e) => {
                // RPC call failed with error
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
    let _governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            // Allow RATE_LIMIT_BURST_SIZE requests per RATE_LIMIT_PERIOD_SECS window per IP.
            // One token is replenished every (period / burst) seconds.
            .per_second(
                crate::constants::RATE_LIMIT_PERIOD_SECS / crate::constants::RATE_LIMIT_BURST_SIZE as u64,
            )
            .burst_size(crate::constants::RATE_LIMIT_BURST_SIZE)
            .error_handler(|_| {
                // Return a JSON 429 response matching the standard ApiResponse error envelope.
                (
                    axum::http::StatusCode::TOO_MANY_REQUESTS,
                    axum::Json(serde_json::json!({
                        "status": "error",
                        "error": "Too many requests, please try again later."
                    })),
                )
                    .into_response()
            })
            .finish()
            .unwrap(),
    );

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
        .route("/health", get(health))
        .route("/health/detailed", get(health_detailed))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics))
        .with_state(state)
        .nest(
            "/api",
            crate::routes::router(
                config.clone(),
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
        /*
        .layer(GovernorLayer {
            config: governor_conf,
        })
        */
        .layer(build_cors(&config))
        .layer(LoggingLayer)
}

/// Build the Axum router with a live database pool.
fn build_router_with_db(
    config: Config,
    cache: crate::price_cache::PriceCache,
    redis: crate::redis_cache::RedisCache,
    pool: sqlx::PgPool,
    event_bus: crate::ws::EventBus,
) -> Router {
    let _governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            // Allow RATE_LIMIT_BURST_SIZE requests per RATE_LIMIT_PERIOD_SECS window per IP.
            // One token is replenished every (period / burst) seconds.
            .per_second(
                crate::constants::RATE_LIMIT_PERIOD_SECS / crate::constants::RATE_LIMIT_BURST_SIZE as u64,
            )
            .burst_size(crate::constants::RATE_LIMIT_BURST_SIZE)
            .error_handler(|_| {
                // Return a JSON 429 response matching the standard ApiResponse error envelope.
                (
                    axum::http::StatusCode::TOO_MANY_REQUESTS,
                    axum::Json(serde_json::json!({
                        "status": "error",
                        "error": "Too many requests, please try again later."
                    })),
                )
                    .into_response()
            })
            .finish()
            .unwrap(),
    );

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

    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/health/detailed", get(health_detailed))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics))
        .with_state(state)
        .nest(
            "/api",
            crate::routes::router_with_db(
                config.clone(),
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
        /*
        .layer(GovernorLayer {
            config: governor_conf,
        })
        */
        .layer(build_cors(&config))
        .layer(LoggingLayer)
}

/// Initialise all dependencies, build the router, and start serving.
///
/// This is the main server entry point called from `main()`.  It
/// creates the database pool, spawns background workers, initialises
/// the Redis cache, and binds the TCP listener.
pub async fn run(config: Config) {
    let pool = crate::db::create_pool(&config).unwrap_or_else(|error| {
        error!(error = %error, "failed to initialize PostgreSQL pool");
        std::process::exit(1);
    });

    let cache = crate::price_cache::PriceCache::new();
    crate::price_cache::spawn_fetcher(cache.clone());

    let event_bus = crate::ws::EventBus::new();

    // Spawn the on-chain event listener to keep pool and prediction indexes in sync.
    crate::worker::stellar_listener::spawn(
        config.stellar_rpc_url.clone(),
        pool.clone(),
        event_bus.clone(),
        std::time::Duration::from_secs(config.rpc_timeout_secs),
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
