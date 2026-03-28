//! # predifi-backend
//!
//! A minimal Axum HTTP server that demonstrates the [`logging`] middleware.
//!
//! Run with:
//! ```bash
//! cargo run
//! ```
//! Then in another terminal:
//! ```bash
//! curl http://localhost:3000/
//! curl http://localhost:3000/health
//! curl http://localhost:3000/missing   # produces a 404
//! ```
//! You should see log lines like:
//! ```text
//! [REQ] GET / → 200 OK (0ms)
//! [REQ] GET /health → 200 OK (0ms)
//! [REQ] GET /missing → 404 Not Found (0ms)
//! ```

pub mod request_logger;

use axum::{routing::get, Json, Router};
use request_logger::LoggingLayer;
use serde_json::json;

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
async fn root() -> Json<serde_json::Value> {
    Json(json!({ "message": "Welcome to the request-logger demo" }))
}

/// Build the Axum router with the logging middleware attached.
///
/// Keeping router construction in its own function makes it easy to reuse
/// in tests without binding to a real TCP port.
pub fn build_router() -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        // `.layer()` wraps every route in this router with the middleware.
        // Layers are applied from bottom to top, so LoggingLayer is the
        // outermost wrapper — it sees every request first and every
        // response last.
        .layer(LoggingLayer)
}

#[tokio::main]
async fn main() {
    let app = build_router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind to port 3000");

    println!("Listening on http://0.0.0.0:3000");

    axum::serve(listener, app)
        .await
        .expect("server error");
}

mod tests;
