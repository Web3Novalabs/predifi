use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};
use std::net::SocketAddr;
use tracing_subscriber;

mod config;
mod db;
use config::db_config::DbConfig;
use db::database::Database;

#[derive(Clone)]
struct AppState {
    db: Database,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let config = DbConfig::from_env();
    let db = Database::connect(&config).await;
    // Check DB connection at startup
    match db.ping().await {
        Ok(_) => tracing::info!("✅ Connected to PostgreSQL database!"),
        Err(e) => tracing::error!("❌ Failed to connect to database: {}", e),
    }
    let state = AppState { db };

    let app = Router::new()
        .route("/ping", get(ping_handler))
        .route("/health", get(health_handler))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ping_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.ping().await {
        Ok(val) => format!("pong: {}", val),
        Err(e) => format!("db error: {}", e),
    }
}

async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.ping().await {
        Ok(_) => (StatusCode::OK, "ok"),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "db unavailable"),
    }
}
