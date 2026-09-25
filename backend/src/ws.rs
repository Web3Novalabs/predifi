//! WebSocket broadcast for live prediction events.
//!
//! Clients connect at `GET /api/v1/ws?address=<wallet>` with a valid access JWT
//! in the `Authorization: Bearer <token>` header. Query-parameter tokens are
//! accepted only outside production (they leak into access logs). Subscribers
//! receive events whose `user_address` matches the subscribed wallet.
//! Omitting `address` delivers all events (useful for dashboards). The indexer
//! calls [`EventBus::send`] whenever a new prediction is indexed.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

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
use crate::constants::{RATE_LIMIT_WS_BURST, RATE_LIMIT_WS_PERIOD_SECS};
use crate::jwt::{extract_bearer_token, verify_jwt_token_strict};

const CHANNEL_CAPACITY: usize = 256;

/// Maximum allowed message size in bytes to prevent memory exhaustion attacks.
const MAX_MESSAGE_SIZE: usize = 1_048_576; // 1 MB

/// Maximum active concurrent WebSocket connections allowed before returning 429.
const MAX_ACTIVE_CONNECTIONS: usize = 10_000;

/// Per-IP handshake attempts allowed inside [`CONNECT_WINDOW`].
const MAX_CONNECT_ATTEMPTS_PER_IP: usize = 10;
const CONNECT_WINDOW: Duration = Duration::from_secs(60);

/// RAII guard to safely track and decrement active connections on drop or panic.
pub struct ConnectionGuard(Arc<AtomicUsize>);

impl ConnectionGuard {
    pub fn new(active_connections: Arc<AtomicUsize>) -> Self {
        let count = active_connections.fetch_add(1, Ordering::Relaxed) + 1;
        tracing::info!(active_connections = count, "websocket client connected");
        Self(active_connections)
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let count = self.0.fetch_sub(1, Ordering::Relaxed) - 1;
        tracing::info!(active_connections = count, "websocket client disconnected");
    }
}

/// Sliding-window limiter for WebSocket handshake attempts per client IP.
struct ConnectRateLimiter {
    attempts: Mutex<HashMap<String, Vec<Instant>>>,
}

impl ConnectRateLimiter {
    fn new() -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
        }
    }

    /// Returns `true` if this IP is allowed another handshake.
    fn check(&self, ip: &str) -> bool {
        let now = Instant::now();
        let mut map = self.attempts.lock().unwrap_or_else(|e| e.into_inner());
        if map.len() > 10_000 {
            map.retain(|_, stamps| stamps.iter().any(|t| now.duration_since(*t) < CONNECT_WINDOW));
        }
        let entry = map.entry(ip.to_string()).or_default();
        entry.retain(|t| now.duration_since(*t) < CONNECT_WINDOW);
        if entry.len() >= MAX_CONNECT_ATTEMPTS_PER_IP {
            return false;
        }
        entry.push(now);
        true
    }
}

/// Optional query parameters for the WebSocket endpoint.
#[derive(Debug, Deserialize, Default)]
pub struct WsConnectParams {
    /// When set, only events whose `user_address` equals this value are forwarded.
    pub address: Option<String>,
    /// Optional JWT passed as a query parameter when headers are unavailable.
    /// Ignored in production — tokens in URLs leak into access logs and history.
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
    connect_limiter: Arc<ConnectRateLimiter>,
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
            connect_limiter: Arc::new(ConnectRateLimiter::new()),
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

    fn allow_connect(&self, ip: &str) -> bool {
        self.connect_limiter.check(ip)
    }
}

/// Returns `true` when `json` should be delivered to a subscriber with `wallet_filter` and `pool_filter`.
///
/// When filters are `None`, all well-formed events are delivered.
pub fn should_deliver_event(
    json: &str,
    wallet_filter: Option<&str>,
    pool_filter: Option<u64>,
) -> bool {
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

fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn extract_ws_token(
    headers: &HeaderMap,
    params: &WsConnectParams,
    production: bool,
) -> Option<String> {
    let header_token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(extract_bearer_token)
        .map(str::to_string);

    if production {
        return header_token;
    }

    header_token.or_else(|| params.token.clone())
}

/// Validate the Origin header to prevent CSRF-like WebSocket hijacking.
///
/// In production the allow-list is mandatory (enforced at config load). A
/// missing or disallowed Origin is rejected. Outside production an empty
/// allow-list remains permissive for local tooling.
fn validate_origin(headers: &HeaderMap, config: &Config) -> bool {
    if config.allowed_ws_origins.is_empty() {
        return !config.is_production();
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

    if origin.eq_ignore_ascii_case("null") {
        return false;
    }

    config.allowed_ws_origins.iter().any(|allowed| allowed == origin)
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

    let ip = client_ip(&headers);
    if !bus.allow_connect(&ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            axum::Json(serde_json::json!({
                "error": "websocket connection rate limit exceeded",
            })),
        )
            .into_response();
    }

    // Enforce active connection cap to prevent resource exhaustion / DoS
    if bus.active_connections() >= MAX_ACTIVE_CONNECTIONS {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            axum::Json(serde_json::json!({
                "error": "maximum active websocket connections reached",
            })),
        )
            .into_response();
    }

    let Some(token) = extract_ws_token(&headers, &params, config.is_production()) else {
        return unauthorized_response("missing or invalid authorization token");
    };

    let claims = match verify_jwt_token_strict(
        &token,
        &config.secret_key,
        "access",
        config.jwt_key_version,
    ) {
        Ok(claims) => claims,
        Err(error) => return unauthorized_response(&error.to_string()),
    };

    // Authorize wallet subscription: user can only subscribe to their own address
    if let Some(ref target_address) = params.address {
        if target_address != &claims.sub {
            return (
                StatusCode::FORBIDDEN,
                axum::Json(serde_json::json!({
                    "error": "unauthorized wallet subscription",
                })),
            )
                .into_response();
        }
    }

    let wallet_filter = params.address;
    let pool_filter = params.pool_id;
    let span = info_span!(
        "websocket.connect",
        wallet = ?wallet_filter,
        pool_id = ?pool_filter,
        subject = %claims.sub
    );
    ws.on_upgrade(move |socket| {
        handle_socket(socket, bus, wallet_filter, pool_filter).instrument(span)
    })
}

async fn handle_socket(
    mut socket: WebSocket,
    bus: EventBus,
    wallet_filter: Option<String>,
    pool_filter: Option<u64>,
) {
    let mut rx = bus.subscribe();
    let _guard = ConnectionGuard::new(bus.active_connections.clone());

    run_socket(&mut socket, &mut rx, wallet_filter.as_deref(), pool_filter).await;

    let _ = socket.send(Message::Close(None)).await;
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
        self.timestamps
            .retain(|t| now.duration_since(*t) < self.window_size);
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
    let mut rate_limiter = WsRateLimiter::new(RATE_LIMIT_WS_BURST, RATE_LIMIT_WS_PERIOD_SECS);

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if !should_deliver_event(&msg, wallet_filter, pool_filter) {
                            continue;
                        }
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
                let msg = match msg {
                    Some(Ok(m)) => m,
                    _ => break,
                };

                match &msg {
                    Message::Text(text) => {
                        if text.len() > MAX_MESSAGE_SIZE {
                            tracing::warn!(
                                message_size = text.len(),
                                max_size = MAX_MESSAGE_SIZE,
                                "Incoming WebSocket text message exceeds size limit, closing connection"
                            );
                            let _ = socket.send(Message::Close(None)).await;
                            break;
                        }
                    }
                    Message::Binary(data) => {
                        if data.len() > MAX_MESSAGE_SIZE {
                            tracing::warn!(
                                message_size = data.len(),
                                max_size = MAX_MESSAGE_SIZE,
                                "Incoming WebSocket binary message exceeds size limit, closing connection"
                            );
                            let _ = socket.send(Message::Close(None)).await;
                            break;
                        }
                    }
                    Message::Ping(payload) => {
                        let _ = socket.send(Message::Pong(payload.clone())).await;
                        continue;
                    }
                    Message::Pong(_) => continue,
                    Message::Close(_) => break,
                }

                if !rate_limiter.check() {
                    tracing::warn!("WebSocket connection rate limited");
                    let _ = socket.send(Message::Close(None)).await;
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwt::{sign_jwt_for_test, sign_jwt_with_type, verify_jwt_token};

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

    #[test]
    fn connection_guard_tracks_and_decrements_atomic_counter() {
        let counter = Arc::new(AtomicUsize::new(0));
        assert_eq!(counter.load(Ordering::Relaxed), 0);

        {
            let _guard = ConnectionGuard::new(counter.clone());
            assert_eq!(counter.load(Ordering::Relaxed), 1);
        }

        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn extract_ws_token_prefers_authorization_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer header-jwt-token".parse().unwrap(),
        );

        let params = WsConnectParams {
            token: Some("query-param-token".to_string()),
            ..Default::default()
        };

        let token = extract_ws_token(&headers, &params, false);
        assert_eq!(token, Some("header-jwt-token".to_string()));
    }

    #[test]
    fn extract_ws_token_ignores_query_param_in_production() {
        let headers = HeaderMap::new();
        let params = WsConnectParams {
            token: Some("query-param-token".to_string()),
            ..Default::default()
        };
        assert!(extract_ws_token(&headers, &params, true).is_none());
    }

    #[test]
    fn validate_origin_allows_matching_origins_and_blocks_disallowed() {
        let mut config = Config::default_for_test();
        config.allowed_ws_origins = vec!["https://app.predifi.com".to_string()];

        let mut valid_headers = HeaderMap::new();
        valid_headers.insert("origin", "https://app.predifi.com".parse().unwrap());
        assert!(validate_origin(&valid_headers, &config));

        let mut invalid_headers = HeaderMap::new();
        invalid_headers.insert("origin", "https://malicious.com".parse().unwrap());
        assert!(!validate_origin(&invalid_headers, &config));

        let empty_headers = HeaderMap::new();
        assert!(!validate_origin(&empty_headers, &config));

        let mut null_origin = HeaderMap::new();
        null_origin.insert("origin", "null".parse().unwrap());
        assert!(!validate_origin(&null_origin, &config));
    }

    #[test]
    fn validate_origin_fails_closed_in_production_without_allow_list() {
        let mut config = Config::default_for_test();
        config.app_env = "production".to_string();
        config.allowed_ws_origins = Vec::new();
        let mut headers = HeaderMap::new();
        headers.insert("origin", "https://app.predifi.com".parse().unwrap());
        assert!(!validate_origin(&headers, &config));
    }

    #[test]
    fn connect_rate_limiter_blocks_after_burst() {
        let limiter = ConnectRateLimiter::new();
        for _ in 0..MAX_CONNECT_ATTEMPTS_PER_IP {
            assert!(limiter.check("1.2.3.4"));
        }
        assert!(!limiter.check("1.2.3.4"));
        assert!(limiter.check("5.6.7.8"), "other IPs are independent");
    }

    #[test]
    fn refresh_token_is_rejected_for_websocket_access() {
        let secret = "predifi-dev-secret-do-not-use-in-production-32";
        let token = sign_jwt_with_type("GABC123", secret, 1_800_000_000, "refresh", 0).unwrap();
        let error = verify_jwt_token_strict(&token, secret, "access", 0).unwrap_err();
        assert_eq!(error, crate::jwt::JwtVerifyError::WrongTokenType);
    }

    #[test]
    fn client_ip_prefers_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "10.0.0.1, 10.0.0.2".parse().unwrap());
        headers.insert("x-real-ip", "9.9.9.9".parse().unwrap());
        assert_eq!(client_ip(&headers), "10.0.0.1");
    }
}
