//! # predifi-backend
//!
//! A minimal Axum HTTP server with CORS and request-logging middleware.

pub mod config;
pub mod db;
pub mod request_logger;
pub mod routes;
pub mod worker;

use axum::{routing::get, Json, Router};
use config::Config;
use http::HeaderValue;
use request_logger::LoggingLayer;
use serde_json::json;
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

/// Health-check handler.
async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "predifi-backend",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Root handler — returns a welcome message.
async fn root() -> Json<serde_json::Value> {
    Json(json!({
        "message": "Welcome to the PrediFi backend",
        "api": "/api/v1"
    }))
}

/// Build the Axum router with CORS and logging middleware attached.
pub fn build_router(config: Config) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .nest("/api", routes::router(config))
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

    let _pool = db::create_pool(&config).unwrap_or_else(|error| {
        error!(error = %error, "failed to initialize PostgreSQL pool");
        std::process::exit(1);
    });

    worker::stellar_listener::spawn(config.stellar_rpc_url.clone(), _pool.clone());

    let app = build_router(config.clone());

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
