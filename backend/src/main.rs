//! # predifi-backend
//!
//! A minimal Axum HTTP server with CORS and request-logging middleware.

pub mod config;
pub mod db;
pub mod openapi;
pub mod price_cache;
pub mod referrals;
pub mod request_logger;
pub mod response;
pub mod routes;
pub mod worker;

use axum::{routing::get, Json, Router, response::IntoResponse};
use std::net::SocketAddr;
use config::Config;
use http::HeaderValue;
use request_logger::LoggingLayer;
use serde_json::json;
use tower_http::cors::{AllowOrigin, CorsLayer};
use std::sync::Arc;
use tower_governor::{
    governor::GovernorConfigBuilder,
    GovernorLayer
};
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
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let rpc_req = client.post(&state.config.stellar_rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getHealth"
        }))
        .send()
        .await;

    match rpc_req {
        Ok(res) if res.status().is_success() => {}
        _ => {
            rpc_status = "unreachable";
            all_healthy = false;
        }
    }

    let body = json!({
        "status": if all_healthy { "ok" } else { "error" },
        "service": "predifi-backend",
        "version": env!("CARGO_PKG_VERSION"),
        "dependencies": {
            "db": db_status,
            "rpc": rpc_status
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


/// Build the Axum router with CORS, logging, and rate limiting middleware.
pub fn build_router(config: Config, cache: price_cache::PriceCache) -> Router {
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(5)
            .burst_size(50)
            .error_handler(|_| {
                (axum::http::StatusCode::TOO_MANY_REQUESTS, "Too Many Requests").into_response()
            })
            .finish()
            .unwrap(),
    );

    let state = routes::v1::AppState {
        config: config.clone(),
        cache: cache.clone(),
        db: None,
    };

    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .with_state(state)
        .nest("/api", routes::router(config, cache, None))
        .merge(openapi::swagger_router())
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
    pool: sqlx::PgPool,
) -> Router {
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(5)
            .burst_size(50)
            .error_handler(|_| {
                (axum::http::StatusCode::TOO_MANY_REQUESTS, "Too Many Requests").into_response()
            })
            .finish()
            .unwrap(),
    );

    let state = routes::v1::AppState {
        config: config.clone(),
        cache: cache.clone(),
        db: Some(pool.clone()),
    };

    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .with_state(state)
        .nest("/api", routes::router_with_db(config, cache, pool))
        // Swagger UI served at /swagger-ui/ (#563)
        .merge(openapi::swagger_router())
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

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(config.log_level.clone()))
        .with_target(false)
        .compact()
        .init();

    let pool = db::create_pool(&config).unwrap_or_else(|error| {
        error!(error = %error, "failed to initialize PostgreSQL pool");
        std::process::exit(1);
    });

    let cache = price_cache::PriceCache::new();
    price_cache::spawn_fetcher(cache.clone());

    let app = build_router_with_db(config.clone(), cache, pool);

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
mod tests;
#[cfg(test)]
mod db_integration_tests;
