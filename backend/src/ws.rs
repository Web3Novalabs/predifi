//! WebSocket broadcast for live prediction events.
//!
//! Clients connect at `GET /api/v1/ws?address=<wallet>` with a valid JWT in the
//! `Authorization: Bearer <token>` header (or `?token=<jwt>` query param) to
//! receive only events where `user_address` matches the subscribed wallet.
//! Omitting `address` delivers all events (useful for dashboards). The indexer
//! calls [`EventBus::send`] whenever a new prediction is indexed.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Instant;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::broadcast;
use tracing::info_span;
use tracing::Instrument;

use crate::config::Config;
use crate::jwt::{extract_bearer_token, verify_jwt_token};

const CHANNEL_CAPACITY: usize = 256;

/// Maximum allowed message size in bytes to prevent memory exhaustion attacks.
const MAX_MESSAGE_SIZE: usize = 1_048_576; // 1 MB

/// Optional query parameters for the WebSocket endpoint.
#[derive(Debug, Deserialize, Default)]
pub struct WsConnectParams {
    /// When set, only events whose `user_address` equals this value are forwarded.
    pub address: Option<String>,
    /// Optional JWT passed as a query parameter when headers are unavailable.
    pub token: Option<String>,
    /// When set, only events whose `pool_id` equals this value are forwarded.
    pub pool_id: Option<u64>,
}

/// Shareable handle to the broadcast channel.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<String>,
    /// Number of currently connected WebSocket clients.
    active_connections: Arc<AtomicUsize>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Create a new broadcast channel with a capacity of [`CHANNEL_CAPACITY`] messages.
    ///
    /// Lagging receivers (slow clients) will have messages dropped rather than
    /// blocking the sender.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            tx,
            active_connections: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Number of WebSocket clients currently connected.
    pub fn active_connections(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
    }

    /// Publish a serialisable event to all connected WebSocket clients.
    /// Silently drops the message if there are no subscribers.
    pub fn send<T: Serialize>(&self, event: &T) {
        if let Ok(json) = serde_json::to_string(event) {
            let _ = self.tx.send(json);
        }
    }

    /// Subscribe to the broadcast channel.
    ///
    /// Each call returns an independent [`broadcast::Receiver`] that will
    /// receive every message published after the subscription is created.
    /// Receivers that fall more than [`CHANNEL_CAPACITY`] messages behind
    /// will receive a [`broadcast::error::RecvError::Lagged`] error.
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }
}

/// Returns `true` when `json` should be delivered to a subscriber with `wallet_filter` and `pool_filter`.
///
/// When filters are `None`, all well-formed events are delivered.
pub fn should_deliver_event(json: &str, wallet_filter: Option<&str>, pool_filter: Option<u64>) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return false;
    };

    if let Some(wallet) = wallet_filter {
        if value.get("user_address").and_then(|v| v.as_str()) != Some(wallet) {
            return false;
        }
    }

    if let Some(pool_id) = pool_filter {
        if value.get("pool_id").and_then(|v| v.as_u64()) != Some(pool_id) {
            return false;
        }
    }

    true
}

fn extract_ws_token(headers: &HeaderMap, params: &WsConnectParams) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(extract_bearer_token)
        .map(str::to_string)
        .or_else(|| params.token.clone())
}

/// Validate the Origin header to prevent CSRF-like WebSocket hijacking.
///
/// This checks that the Origin header matches the configured allowed origins.
/// If no origins are configured, the check is skipped (allowing any origin).
/// In production, this should be configured to restrict to trusted domains.
fn validate_origin(headers: &HeaderMap, config: &Config) -> bool {
    // If no allowed origins are configured, skip validation (permissive mode)
    if config.allowed_ws_origins.is_empty() {
        return true;
    }

    let origin = match headers.get("origin") {
        Some(value) => match value.to_str() {
            Ok(origin_str) => origin_str,
            Err(_) => {
                tracing::warn!("Invalid Origin header format");
                return false;
            }
        },
        None => {
            tracing::warn!("Missing Origin header in WebSocket upgrade request");
            return false;
        }
    };

    // Check if the origin is in the allowed list
    config.allowed_ws_origins.contains(&origin.to_string())
}

fn unauthorized_response(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        axum::Json(serde_json::json!({
            "error": message,
        })),
    )
        .into_response()
}

/// Axum handler — validates JWT credentials, then upgrades to WebSocket.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    Query(params): Query<WsConnectParams>,
    State(config): State<Arc<Config>>,
    State(bus): State<EventBus>,
) -> impl IntoResponse {
    // Validate Origin header to prevent CSRF-like hijacking
    if !validate_origin(&headers, &config) {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "error": "invalid or missing Origin header",
            })),
        )
            .into_response();
    }

    let Some(token) = extract_ws_token(&headers, &params) else {
        return unauthorized_response("missing or invalid authorization token");
    };

    let claims = match verify_jwt_token(&token, &config.secret_key) {
        Ok(claims) => claims,
        Err(error) => return unauthorized_response(&error.to_string()),
    };

    let wallet_filter = params.address;
    let pool_filter = params.pool_id;
    let span = info_span!(
        "websocket.connect",
        wallet = ?wallet_filter,
        pool_id = ?pool_filter,
        subject = %claims.sub
    );
    ws.on_upgrade(move |socket| handle_socket(socket, bus, wallet_filter, pool_filter).instrument(span))
}

async fn handle_socket(mut socket: WebSocket, bus: EventBus, wallet_filter: Option<String>, pool_filter: Option<u64>) {
    let mut rx = bus.subscribe();

    let count = bus.active_connections.fetch_add(1, Ordering::Relaxed) + 1;
    tracing::info!(
        active_connections = count,
        wallet = ?wallet_filter,
        pool_id = ?pool_filter,
        "websocket client connected"
    );

    run_socket(&mut socket, &mut rx, wallet_filter.as_deref(), pool_filter).await;

    let count = bus.active_connections.fetch_sub(1, Ordering::Relaxed) - 1;
    tracing::info!(active_connections = count, "websocket client disconnected");
}

/// Per-connection message rate limiter using a sliding window.
struct WsRateLimiter {
    window_size: std::time::Duration,
    max_messages: u32,
    timestamps: Vec<Instant>,
}

impl WsRateLimiter {
    fn new(max_messages: u32, window_secs: u64) -> Self {
        Self {
            window_size: std::time::Duration::from_secs(window_secs),
            max_messages,
            timestamps: Vec::with_capacity(max_messages as usize + 1),
        }
    }

    /// Returns `true` if the message is allowed, `false` if rate-limited.
    fn check(&mut self) -> bool {
        let now = Instant::now();
        // Remove timestamps outside the window
        self.timestamps.retain(|t| now.duration_since(*t) < self.window_size);
        if self.timestamps.len() >= self.max_messages as usize {
            return false;
        }
        self.timestamps.push(now);
        true
    }
}

async fn run_socket(
    socket: &mut WebSocket,
    rx: &mut broadcast::Receiver<String>,
    wallet_filter: Option<&str>,
    pool_filter: Option<u64>,
) {
    let mut rate_limiter = WsRateLimiter::new(10, 10);

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if !should_deliver_event(&msg, wallet_filter, pool_filter) {
                            continue;
                        }
                        // Enforce message size limit to prevent memory exhaustion
                        if msg.len() > MAX_MESSAGE_SIZE {
                            tracing::warn!(
                                message_size = msg.len(),
                                max_size = MAX_MESSAGE_SIZE,
                                "WebSocket message exceeds size limit, dropping"
                            );
                            continue;
                        }
                        if socket.send(Message::Text(msg)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            msg = socket.recv() => {
                if msg.is_none() { break; }
                if let Some(Ok(message)) = msg {
                    match message {
                        Message::Text(text) => {
                            // Enforce message size limit
                            if text.len() > MAX_MESSAGE_SIZE {
                                tracing::warn!(
                                    message_size = text.len(),
                                    max_size = MAX_MESSAGE_SIZE,
                                    "Incoming WebSocket message exceeds size limit, closing connection"
                                );
                                break;
                            }
                            // Per-message rate limiting: disconnect if client sends too many messages
                            if !rate_limiter.check() {
                                tracing::warn!(
                                    wallet = ?wallet_filter,
                                    "WebSocket client exceeded message rate limit, closing connection"
                                );
                                let _ = socket.send(Message::Text(
                                    r#"{"error":"rate_limit_exceeded","message":"too many messages"}"#.into()
                                )).await;
                                break;
                            }
                        }
                        Message::Ping(data) => {
                            if socket.send(Message::Pong(data)).await.is_err() {
                                break;
                            }
                        }
                        Message::Close(_) => break,
                        _ => {}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::should_deliver_event;
    use crate::jwt::{sign_jwt_for_test, verify_jwt_token};

    #[test]
    fn delivers_all_events_when_no_wallet_filter() {
        let json = r#"{"type":"prediction_placed","user_address":"GABC","pool_id":1}"#;
        assert!(should_deliver_event(json, None, None));
    }

    #[test]
    fn delivers_event_when_wallet_matches() {
        let json = r#"{"type":"prediction_placed","user_address":"GABC","pool_id":1}"#;
        assert!(should_deliver_event(json, Some("GABC"), None));
    }

    #[test]
    fn skips_event_when_wallet_mismatch() {
        let json = r#"{"type":"prediction_placed","user_address":"GABC","pool_id":1}"#;
        assert!(!should_deliver_event(json, Some("GXYZ"), None));
    }

    #[test]
    fn skips_malformed_json_when_filter_active() {
        assert!(!should_deliver_event("not-json", Some("GABC"), None));
    }

    #[test]
    fn skips_event_missing_user_address_when_filter_active() {
        let json = r#"{"type":"prediction_placed","pool_id":1}"#;
        assert!(!should_deliver_event(json, Some("GABC"), None));
    }

    #[test]
    fn ws_token_from_query_param_verifies_against_secret() {
        let secret = "predifi-dev-secret-do-not-use-in-production-32";
        let token = sign_jwt_for_test("GABC123", secret);
        let claims = verify_jwt_token(&token, secret).expect("valid token");
        assert_eq!(claims.sub, "GABC123");
    }
}
