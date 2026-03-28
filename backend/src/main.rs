//! # predifi-backend
//!
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

pub mod request_logger;
pub mod routes;

use axum::{routing::get, Json, Router};
use request_logger::LoggingLayer;
use routes::v1;
use serde_json::json;

/// Root handler that points callers at the versioned API namespace.
async fn root() -> Json<serde_json::Value> {
    Json(json!({
        "message": "Welcome to the PrediFi backend",
        "api": "/api/v1"
    }))
}

/// Build the Axum router with the logging middleware attached.
///
/// Keeping router construction in its own function makes it easy to reuse
/// in tests without binding to a real TCP port.
pub fn build_router() -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(v1::health))
        .nest("/api", routes::router())
        .layer(LoggingLayer)
}

#[tokio::main]
async fn main() {
    let app = build_router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind to port 3000");

    println!("Listening on http://0.0.0.0:3000");

    axum::serve(listener, app).await.expect("server error");
}

mod tests;
