use axum::{Router, extract::State, http::HeaderMap, routing::get};
use std::net::SocketAddr;
use tokio::signal;
#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal as unix_signal};
use tower_http::request_id::MakeRequestUuid;
use tracing::Instrument;

mod config;
mod db;
mod error;
use config::db_config::DbConfig;
use config::tracing::{TracingConfig, get_trace_context, init_tracing, shutdown_tracing};
use db::database::Database;
use error::{AppError, AppResult};

#[derive(Clone)]
struct AppState {
    db: Database,
}

#[tokio::main]
async fn main() {
    // Initialize structured logging with OpenTelemetry
    let tracing_config = TracingConfig::from_env();
    if let Err(e) = init_tracing(&tracing_config) {
        eprintln!("Failed to initialize tracing: {e}");
        std::process::exit(1);
    }

    let config = DbConfig::from_env();
    let db = Database::connect(&config).await;

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
            std::process::exit(1);
        }
    }

    let state = AppState { db };

    let app = Router::new()
        .route("/ping", get(ping_handler))
        .route("/health", get(health_handler))
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
) -> AppResult<&'static str> {
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
                Ok("ok")
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
