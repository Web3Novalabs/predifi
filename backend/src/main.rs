//! # predifi-backend
//!
//! A minimal Axum HTTP server with CORS and request-logging middleware.
//! A minimal Axum HTTP server that demonstrates the request-logging middleware
//! and a versioned API router layout.
//!
//! Run with:
//! ```bash
//! cargo run
//! ```
//! Then in another terminal:
//! ```bash
//! curl http://localhost:3000/
//! curl http://localhost:3000/health
//! curl http://localhost:3000/api/v1
//! curl http://localhost:3000/api/v1/health
//! curl http://localhost:3000/missing
//! ```

pub mod config;
pub mod db;
pub mod request_logger;
pub mod routes;

use axum::{routing::get, Json, Router};
use http::HeaderValue;
use config::Config;
use request_logger::LoggingLayer;
use routes::v1;
use serde_json::json;
use tower_http::cors::{AllowOrigin, CorsLayer};

/// Allowed frontend origins for CORS.
///
/// Add any additional frontend URLs here.
/// In production, replace with your actual deployed frontend URL.
const ALLOWED_ORIGINS: &[&str] = &[
    "http://localhost:3000",
    "http://localhost:5173",
    "https://predifi.app",
];

/// Build the CORS middleware layer.
///
/// Only requests from the origins listed in `ALLOWED_ORIGINS` are permitted.
/// All other origins will be rejected by the browser.
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

/// Health-check handler.
///
/// Returns HTTP 200 with basic system info:
/// - `status`: always `"ok"` when the server is running
/// - `service`: name of the service
/// - `version`: current package version from Cargo.toml
async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "predifi-backend",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Root handler — returns a welcome message.
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

/// Health-check handler.
///
/// Returns HTTP 200 with basic system info:
/// - `status`: always `"ok"` when the server is running
/// - `service`: name of the service
/// - `version`: current package version from Cargo.toml
async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "predifi-backend",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Root handler — returns a welcome message.
/// Root handler that points callers at the versioned API namespace.
async fn root() -> Json<serde_json::Value> {
    Json(json!({
        "message": "Welcome to the PrediFi backend",
        "api": "/api/v1"
    }))
}

/// Build the Axum router with CORS and logging middleware attached.
///
/// Keeping router construction in its own function makes it easy to reuse
/// in tests without binding to a real TCP port.
pub fn build_router() -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .layer(build_cors())
        .route("/health", get(v1::health))
        .nest("/api", routes::router())
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

    let _pool = db::create_pool(&config).unwrap_or_else(|error| {
        error!(error = %error, "failed to initialize PostgreSQL pool");
        std::process::exit(1);
    });

    let app = build_router();

    let bind_addr = config.bind_address();

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|error| {
            error!(address = %bind_addr, error = %error, "failed to bind TCP listener");
            std::process::exit(1);
        });

    info!(address = %bind_addr, "backend server listening");

    if let Err(error) = axum::serve(listener, app).await {
        error!(error = %error, "server error");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests;
