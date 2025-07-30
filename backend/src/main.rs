mod config;
mod controllers;
mod db;
pub mod error;
mod models;
mod routes;

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
};

use routes::pool_route::pool_routes;
use std::net::SocketAddr;
use tower_http::request_id::MakeRequestUuid;
use tracing::Instrument;

use config::db_config::DbConfig;
use config::tracing::{TracingConfig, get_trace_context, init_tracing, shutdown_tracing};
use db::database::Database;
use error::{AppError, AppResult};

use db::database::AppState;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Initialize structured logging with OpenTelemetry
    let tracing_config = TracingConfig::from_env();
    init_tracing(&tracing_config).map_err(|e| {
        eprintln!("Failed to initialize tracing: {e}");
        AppError::Internal(format!("Tracing initialization failed: {e}"))
    })?;

    let config = DbConfig::from_env();
    let db = Database::connect(&config).await;
    // Run SQLX migrations after connecting to the database
    sqlx::migrate!("./migrations")
        .run(db.pool())
        .await
        .expect("Failed to run migrations");

    // Check DB connection at startup with structured logging
    match db.ping().await {
        Ok(_) => {
            tracing::info!(
                event = "database_connection_success",
                database.type = "postgresql",
                "Successfully connected to PostgreSQL database"
            );
        }
        Err(e) => {
            tracing::error!(
                event = "database_connection_failed",
                database.type = "postgresql",
                error = %e,
                "Failed to connect to database"
            );
            shutdown_tracing();
            return Err(AppError::Internal(format!(
                "Database connection failed: {e}"
            )));
        }
    }

    let state = AppState { db };

    let app = Router::new()
        .route("/ping", get(ping_handler))
        .route("/health", get(health_handler))
        .merge(pool_routes()) // Merge the new pool routes
        .merge(validator_routes())
        .with_state(state)
        .layer(tower_http::request_id::SetRequestIdLayer::new(
            axum::http::header::HeaderName::from_static("x-request-id"),
            MakeRequestUuid,
        ));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!(
        event = "server_starting",
        server.address = %addr,
        server.port = 3000,
        "Starting server with manual OpenTelemetry trace correlation"
    );

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    // Set up graceful shutdown
    let server = axum::serve(listener, app);

    // Handle shutdown signal
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                tracing::error!(
                    event = "server_error",
                    error = %e,
                    "Server error"
                );
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!(
                event = "shutdown_signal_received",
                "Received shutdown signal"
            );
        }
    }

    tracing::info!(event = "server_shutdown", "Server shutdown complete");
    shutdown_tracing();
    Ok(())
}

async fn ping_handler(State(state): State<AppState>, headers: HeaderMap) -> AppResult<String> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    // Note: OpenTelemetry span creation removed due to dependency version conflicts

    let span = tracing::info_span!(
        "ping_handler",
        http.method = "GET",
        http.route = "/ping",
        request_id = request_id,
    );

    async move {
        // Get trace context for structured logging
        let (trace_id, span_id) =
            get_trace_context().unwrap_or(("unknown".to_string(), "unknown".to_string()));

        tracing::info!(
            event = "http_request_start",
            http.method = "GET",
            http.route = "/ping",
            request_id = request_id,
            otel.trace_id = trace_id,
            otel.span_id = span_id,
            "Processing ping request with OpenTelemetry correlation"
        );

        let start_time = std::time::Instant::now();

        match state.db.ping().await {
            Ok(val) => {
                let response = format!("pong: {val}");
                let (success_trace_id, success_span_id) =
                    get_trace_context().unwrap_or(("unknown".to_string(), "unknown".to_string()));

                tracing::info!(
                    event = "http_request_success",
                    http.method = "GET",
                    http.route = "/ping",
                    http.status_code = 200,
                    http.response_time_ms = start_time.elapsed().as_millis(),
                    database.response = val,
                    request_id = request_id,
                    otel.trace_id = success_trace_id,
                    otel.span_id = success_span_id,
                    "Ping request successful with OpenTelemetry correlation"
                );
                Ok(response)
            }
            Err(e) => {
                let (error_trace_id, error_span_id) =
                    get_trace_context().unwrap_or(("unknown".to_string(), "unknown".to_string()));

                tracing::error!(
                    event = "http_request_error",
                    http.method = "GET",
                    http.route = "/ping",
                    http.status_code = 500,
                    http.response_time_ms = start_time.elapsed().as_millis(),
                    request_id = request_id,
                    otel.trace_id = error_trace_id,
                    otel.span_id = error_span_id,
                    error = %e,
                    "Ping request failed due to database error with OpenTelemetry correlation"
                );
                let app_error = AppError::database_with_context(e, "ping request");
                Err(app_error)
            }
        }
    }
    .instrument(span)
    .await
}

async fn health_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl IntoResponse> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    // Note: OpenTelemetry span creation removed due to dependency version conflicts

    let span = tracing::info_span!(
        "health_handler",
        http.method = "GET",
        http.route = "/health",
        request_id = request_id,
    );

    async move {
        // Get trace context for structured logging
        let (trace_id, span_id) =
            get_trace_context().unwrap_or(("unknown".to_string(), "unknown".to_string()));

        tracing::info!(
            event = "http_request_start",
            http.method = "GET",
            http.route = "/health",
            request_id = request_id,
            otel.trace_id = trace_id,
            otel.span_id = span_id,
            "Processing health check request with OpenTelemetry correlation"
        );

        let start_time = std::time::Instant::now();

        match state.db.ping().await {
            Ok(_) => {
                let (success_trace_id, success_span_id) =
                    get_trace_context().unwrap_or(("unknown".to_string(), "unknown".to_string()));

                tracing::info!(
                    event = "http_request_success",
                    http.method = "GET",
                    http.route = "/health",
                    http.status_code = 200,
                    http.response_time_ms = start_time.elapsed().as_millis(),
                    health.status = "healthy",
                    request_id = request_id,
                    otel.trace_id = success_trace_id,
                    otel.span_id = success_span_id,
                    "Health check passed with OpenTelemetry correlation"
                );
                Ok((StatusCode::OK, "ok"))
            }
            Err(e) => {
                let (error_trace_id, error_span_id) =
                    get_trace_context().unwrap_or(("unknown".to_string(), "unknown".to_string()));

                tracing::error!(
                    event = "http_request_error",
                    http.method = "GET",
                    http.route = "/health",
                    http.status_code = 500,
                    http.response_time_ms = start_time.elapsed().as_millis(),
                    health.status = "unhealthy",
                    request_id = request_id,
                    otel.trace_id = error_trace_id,
                    otel.span_id = error_span_id,
                    error = %e,
                    "Health check failed due to database error with OpenTelemetry correlation"
                );
                let app_error = AppError::database_with_context(e, "health check");
                Err(app_error)
            }
        }
    }
    .instrument(span)
    .await
}
