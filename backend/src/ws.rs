//! WebSocket broadcast for live prediction events.
//!
//! Clients connect at `GET /api/v1/ws?address=<wallet>` with a valid access JWT
//! in the `Authorization: Bearer <token>` header. Query-parameter tokens are
//! accepted only outside production (they leak into access logs). Subscribers
//! receive events whose `user_address` matches the subscribed wallet.
//! Omitting `address` delivers all events (useful for dashboards). The indexer
//! calls [`EventBus::send`] whenever a new prediction is indexed.
//!
//! # High-throughput delivery (#1941)
//!
//! Events are serialised **once** per publish into a [`BusEvent`] that carries
//! its routing keys (`pool_id`, `user_address`, `type`) next to the JSON
//! payload, so filtering a subscriber is a field comparison instead of a JSON
//! parse per event per connection.
//!
//! ## Selective subscriptions
//!
//! A connection only receives events for the pools it is interested in. The
//! initial set comes from the query string (`pool_ids=1,2,3`, or the legacy
//! single `pool_id=1`) and can be changed at runtime with control frames:
//!
//! ```json
//! {"action": "subscribe",   "pool_ids": [4, 5]}
//! {"action": "unsubscribe", "pool_ids": [1]}
//! {"action": "list"}
//! ```
//!
//! Each control frame is answered with
//! `{"type":"subscriptions","pool_ids":[...],"events":[...]}`. An empty pool
//! set means "all pools". `events=prediction_placed,pool_resolved` restricts
//! delivery to the listed event types.
//!
//! ## Batching
//!
//! `batch_ms=<1..=1000>` buffers matching events for up to that many
//! milliseconds (or `batch_max` events, default 50) and sends them as one
//! frame: `{"type":"batch","count":N,"events":[...]}`. Without `batch_ms`
//! every event is sent as its own frame, exactly as before.
//!
//! ## Compression
//!
//! `compress=deflate` makes the server zlib-compress frames of at least
//! [`COMPRESSION_THRESHOLD`] bytes and send them as **binary** frames (smaller
//! frames stay text). Browsers can inflate them with
//! `new DecompressionStream("deflate")`.
//!
//! When a slow client falls behind the broadcast buffer it receives
//! `{"type":"lagged","missed":N}` so it knows to resynchronise over REST.

use std::collections::{BTreeSet, HashMap};
use std::io::Write as _;
use std::sync::{Arc, Mutex};
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
use crate::metrics::SharedMetrics;
use crate::shutdown::WsShutdownToken;

/// Broadcast buffer depth. Sized for bursts of indexed events so ordinary
/// clients do not lag; a client that still falls behind is told how many
/// events it missed.
const CHANNEL_CAPACITY: usize = 4_096;

/// Maximum number of pools a single connection may subscribe to.
pub const MAX_POOL_SUBSCRIPTIONS: usize = 100;

/// Maximum number of event-type filters a single connection may set.
const MAX_EVENT_TYPE_FILTERS: usize = 16;

/// Upper bound for the client-requested batching window.
const MAX_BATCH_WINDOW_MS: u64 = 1_000;

/// Default and maximum number of events per batch frame.
const DEFAULT_BATCH_MAX: usize = 50;
const MAX_BATCH_MAX: usize = 500;

/// Frames smaller than this are never compressed — deflate overhead would
/// outweigh the savings.
pub const COMPRESSION_THRESHOLD: usize = 1_024;

/// Maximum allowed message size in bytes to prevent memory exhaustion attacks.
const MAX_MESSAGE_SIZE: usize = 1_048_576; // 1 MB

/// Maximum active concurrent WebSocket connections allowed before returning 429.
const MAX_ACTIVE_CONNECTIONS: usize = 10_000;

/// Per-IP handshake attempts allowed inside [`CONNECT_WINDOW`].
const MAX_CONNECT_ATTEMPTS_PER_IP: usize = 10;
const CONNECT_WINDOW: Duration = Duration::from_secs(60);

/// RAII guard to safely track and decrement active connections on drop or panic.
///
/// Backed by [`WsShutdownToken`] so that the same active-connection count
/// used for the `MAX_ACTIVE_CONNECTIONS` cap also drives graceful shutdown:
/// [`crate::server::run_with_signal`] broadcasts the shutdown signal through
/// this token and waits for the count it tracks to reach zero.
pub struct ConnectionGuard(WsShutdownToken, Option<SharedMetrics>);

impl ConnectionGuard {
    /// Subscribes to `token` (incrementing its active-connection count) and
    /// returns both the RAII guard (which decrements the count on drop) and
    /// the broadcast receiver the connection should select on to learn when
    /// the server is shutting down.
    pub fn subscribe(token: WsShutdownToken) -> (Self, broadcast::Receiver<()>) {
        Self::subscribe_with_metrics(token, None)
    }

    /// Like [`ConnectionGuard::subscribe`], additionally keeping the
    /// `app_ws_connections_*` Prometheus metrics in step with the guard.
    pub fn subscribe_with_metrics(
        token: WsShutdownToken,
        metrics: Option<SharedMetrics>,
    ) -> (Self, broadcast::Receiver<()>) {
        let shutdown_rx = token.subscribe();
        if let Some(metrics) = &metrics {
            metrics.ws_connect();
        }
        tracing::info!(
            active_connections = token.active_count(),
            "websocket client connected"
        );
        (Self(token, metrics), shutdown_rx)
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.0.connection_closed();
        if let Some(metrics) = &self.1 {
            metrics.ws_disconnect();
        }
        tracing::info!(
            active_connections = self.0.active_count(),
            "websocket client disconnected"
        );
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
            map.retain(|_, stamps| {
                stamps
                    .iter()
                    .any(|t| now.duration_since(*t) < CONNECT_WINDOW)
            });
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
    /// Merged with `pool_ids`.
    pub pool_id: Option<u64>,
    /// Comma-separated pool IDs to subscribe to, e.g. `1,2,3`.
    pub pool_ids: Option<String>,
    /// Comma-separated event types to receive, e.g. `prediction_placed`.
    pub events: Option<String>,
    /// Batching window in milliseconds (`1..=1000`). Absent or `0` disables
    /// batching.
    pub batch_ms: Option<u64>,
    /// Maximum events per batch frame (default 50, max 500).
    pub batch_max: Option<usize>,
    /// `deflate` enables compression of large frames.
    pub compress: Option<String>,
}

/// An event as it travels over the bus: serialised once, with the fields
/// subscribers filter on extracted up front.
#[derive(Debug)]
pub struct BusEvent {
    /// The event's `type` field, if any.
    pub event_type: Option<String>,
    /// The event's `pool_id` field, if any.
    pub pool_id: Option<u64>,
    /// The event's `user_address` field, if any.
    pub user_address: Option<String>,
    /// The JSON payload sent to clients.
    pub payload: Arc<str>,
}

impl BusEvent {
    /// Serialise `event` and extract its routing keys.
    pub fn from_serialize<T: Serialize>(event: &T) -> Option<Self> {
        let value = serde_json::to_value(event).ok()?;
        let payload = serde_json::to_string(&value).ok()?;
        Some(Self {
            event_type: value
                .get("type")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            pool_id: value.get("pool_id").and_then(|v| v.as_u64()),
            user_address: value
                .get("user_address")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            payload: payload.into(),
        })
    }
}

/// What one connection wants to receive.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Subscription {
    /// Only events for this wallet (fixed at connect time, JWT-authorised).
    pub wallet: Option<String>,
    /// Only events for these pools; empty means every pool.
    pub pools: BTreeSet<u64>,
    /// Only these event types; empty means every type.
    pub event_types: BTreeSet<String>,
}

impl Subscription {
    /// Returns `true` when `event` should be delivered to this subscriber.
    pub fn matches(&self, event: &BusEvent) -> bool {
        if let Some(wallet) = &self.wallet {
            if event.user_address.as_deref() != Some(wallet.as_str()) {
                return false;
            }
        }
        if !self.pools.is_empty() {
            match event.pool_id {
                Some(id) if self.pools.contains(&id) => {}
                _ => return false,
            }
        }
        if !self.event_types.is_empty() {
            match &event.event_type {
                Some(kind) if self.event_types.contains(kind) => {}
                _ => return false,
            }
        }
        true
    }

    /// Add pools, refusing to grow beyond [`MAX_POOL_SUBSCRIPTIONS`].
    fn add_pools(&mut self, ids: &[u64]) -> Result<(), String> {
        let new: BTreeSet<u64> = ids
            .iter()
            .copied()
            .filter(|id| !self.pools.contains(id))
            .collect();
        if self.pools.len() + new.len() > MAX_POOL_SUBSCRIPTIONS {
            return Err(format!(
                "a connection may subscribe to at most {MAX_POOL_SUBSCRIPTIONS} pools"
            ));
        }
        self.pools.extend(new);
        Ok(())
    }

    fn ack(&self) -> String {
        serde_json::json!({
            "type": "subscriptions",
            "pool_ids": self.pools,
            "events": self.event_types,
        })
        .to_string()
    }
}

/// How events are framed for one connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeliveryOptions {
    /// Buffer events for this long before flushing; `None` sends immediately.
    pub batch_window: Option<Duration>,
    /// Flush a batch as soon as it holds this many events.
    pub batch_max: usize,
    /// Deflate frames of at least [`COMPRESSION_THRESHOLD`] bytes.
    pub compress: bool,
}

impl Default for DeliveryOptions {
    fn default() -> Self {
        Self {
            batch_window: None,
            batch_max: DEFAULT_BATCH_MAX,
            compress: false,
        }
    }
}

/// Parse a comma-separated list of pool IDs.
fn parse_pool_ids(raw: &str) -> Result<Vec<u64>, String> {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<u64>()
                .map_err(|_| format!("invalid pool id: {part}"))
        })
        .collect()
}

/// Build the initial subscription and delivery options from the query string.
fn connection_settings(
    params: &WsConnectParams,
) -> Result<(Subscription, DeliveryOptions), String> {
    let mut subscription = Subscription {
        wallet: params.address.clone(),
        ..Default::default()
    };

    let mut pools = Vec::new();
    if let Some(id) = params.pool_id {
        pools.push(id);
    }
    if let Some(raw) = &params.pool_ids {
        pools.extend(parse_pool_ids(raw)?);
    }
    subscription.add_pools(&pools)?;

    if let Some(raw) = &params.events {
        subscription.event_types = raw
            .split(',')
            .map(str::trim)
            .filter(|kind| !kind.is_empty())
            .map(str::to_string)
            .collect();
        if subscription.event_types.len() > MAX_EVENT_TYPE_FILTERS {
            return Err(format!(
                "at most {MAX_EVENT_TYPE_FILTERS} event types may be requested"
            ));
        }
    }

    let batch_window = match params.batch_ms {
        None | Some(0) => None,
        Some(ms) if ms <= MAX_BATCH_WINDOW_MS => Some(Duration::from_millis(ms)),
        Some(_) => {
            return Err(format!(
                "batch_ms must be between 0 and {MAX_BATCH_WINDOW_MS}"
            ))
        }
    };
    let batch_max = params
        .batch_max
        .unwrap_or(DEFAULT_BATCH_MAX)
        .clamp(1, MAX_BATCH_MAX);
    let compress = match params.compress.as_deref() {
        None | Some("") | Some("none") => false,
        Some("deflate") => true,
        Some(other) => return Err(format!("unsupported compression: {other}")),
    };

    Ok((
        subscription,
        DeliveryOptions {
            batch_window,
            batch_max,
            compress,
        },
    ))
}

/// A control frame sent by the client.
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum ClientCommand {
    Subscribe {
        #[serde(default)]
        pool_ids: Vec<u64>,
    },
    Unsubscribe {
        #[serde(default)]
        pool_ids: Vec<u64>,
    },
    List,
}

/// Apply a client control frame and return the reply to send.
fn handle_client_command(subscription: &mut Subscription, text: &str) -> String {
    let command = match serde_json::from_str::<ClientCommand>(text) {
        Ok(command) => command,
        Err(_) => {
            return serde_json::json!({
                "type": "error",
                "message": "unrecognised control message",
            })
            .to_string()
        }
    };
    match command {
        ClientCommand::Subscribe { pool_ids } => {
            if let Err(message) = subscription.add_pools(&pool_ids) {
                return serde_json::json!({ "type": "error", "message": message }).to_string();
            }
        }
        ClientCommand::Unsubscribe { pool_ids } => {
            for id in pool_ids {
                subscription.pools.remove(&id);
            }
        }
        ClientCommand::List => {}
    }
    subscription.ack()
}

/// Wrap raw JSON payloads in a single batch envelope without re-parsing them.
fn batch_frame(events: &[Arc<BusEvent>]) -> String {
    let body_len: usize = events.iter().map(|e| e.payload.len() + 1).sum();
    let mut frame = String::with_capacity(body_len + 48);
    frame.push_str("{\"type\":\"batch\",\"count\":");
    frame.push_str(&events.len().to_string());
    frame.push_str(",\"events\":[");
    for (i, event) in events.iter().enumerate() {
        if i > 0 {
            frame.push(',');
        }
        frame.push_str(&event.payload);
    }
    frame.push_str("]}");
    frame
}

/// zlib-compress `text`.
pub fn deflate(text: &str) -> std::io::Result<Vec<u8>> {
    let mut encoder = flate2::write::ZlibEncoder::new(
        Vec::with_capacity(text.len() / 2),
        flate2::Compression::fast(),
    );
    encoder.write_all(text.as_bytes())?;
    encoder.finish()
}

/// Turn a serialised frame into a WebSocket message, compressing it when the
/// client opted in, the frame is large enough, and compression actually helps.
/// Returns the message and its encoding label.
fn encode_frame(text: String, compress: bool) -> (Message, &'static str) {
    if compress && text.len() >= COMPRESSION_THRESHOLD {
        if let Ok(bytes) = deflate(&text) {
            if bytes.len() < text.len() {
                return (Message::Binary(bytes), "deflate");
            }
        }
    }
    (Message::Text(text), "text")
}

/// Shareable handle to the broadcast channel.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<Arc<BusEvent>>,
    metrics: Option<SharedMetrics>,
    /// Tracks currently connected WebSocket clients and carries the
    /// graceful-shutdown broadcast signal to each of them (see
    /// [`EventBus::ws_shutdown_token`]).
    ws_shutdown: WsShutdownToken,
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
            metrics: None,
            ws_shutdown: WsShutdownToken::new(1),
            connect_limiter: Arc::new(ConnectRateLimiter::new()),
        }
    }

    /// Attach a metrics sink so connections, frames, bytes and dropped
    /// events are reported to Prometheus.
    pub fn with_metrics(mut self, metrics: SharedMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Number of WebSocket clients currently connected.
    pub fn active_connections(&self) -> usize {
        self.ws_shutdown.active_count()
    }

    /// Clone of the shutdown token used to notify every connected WebSocket
    /// client that the server is shutting down, and to track how many of
    /// them are still draining. Used by [`crate::server::run_with_signal`].
    pub fn ws_shutdown_token(&self) -> WsShutdownToken {
        self.ws_shutdown.clone()
    }

    /// Publish a serialisable event to all connected WebSocket clients.
    /// Silently drops the message if there are no subscribers.
    pub fn send<T: Serialize>(&self, event: &T) {
        if self.tx.receiver_count() == 0 {
            return;
        }
        if let Some(event) = BusEvent::from_serialize(event) {
            let _ = self.tx.send(Arc::new(event));
        }
    }

    /// Subscribe to the broadcast channel.
    ///
    /// Each call returns an independent [`broadcast::Receiver`] that will
    /// receive every message published after the subscription is created.
    /// Receivers that fall more than [`CHANNEL_CAPACITY`] messages behind
    /// will receive a [`broadcast::error::RecvError::Lagged`] error.
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<BusEvent>> {
        self.tx.subscribe()
    }

    fn allow_connect(&self, ip: &str) -> bool {
        self.connect_limiter.check(ip)
    }
}

/// Returns `true` when `json` should be delivered to a subscriber with `wallet_filter` and `pool_filter`.
///
/// When filters are `None`, all well-formed events are delivered. The socket
/// loop itself filters pre-parsed [`BusEvent`]s via [`Subscription::matches`];
/// this helper is kept for callers holding raw JSON.
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
        .and_then(|ip_str| {
            if ip_str.parse::<std::net::IpAddr>().is_ok() {
                Some(ip_str)
            } else {
                None
            }
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

    config
        .allowed_ws_origins
        .iter()
        .any(|allowed| allowed == origin)
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
    if ip == "unknown" {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "unable to determine client IP",
            })),
        )
            .into_response();
    }
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

    let claims =
        match verify_jwt_token_strict(&token, &config.secret_key, "access", config.jwt_key_version)
        {
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

    let (subscription, options) = match connection_settings(&params) {
        Ok(settings) => settings,
        Err(message) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({ "error": message })),
            )
                .into_response()
        }
    };

    crate::tracing_context::record_user_address(&crate::request_logger::redact_address(
        &claims.sub,
    ));
    let span = info_span!(
        "websocket.connect",
        correlation_id = %crate::tracing_context::current_correlation_id()
            .unwrap_or_else(crate::tracing_context::new_correlation_id),
        wallet = ?subscription.wallet.as_deref().map(crate::request_logger::redact_address),
        pools = subscription.pools.len(),
        batch_ms = options.batch_window.map(|w| w.as_millis() as u64),
        compress = options.compress,
        subject = %crate::request_logger::redact_address(&claims.sub)
    );
    ws.on_upgrade(move |socket| handle_socket(socket, bus, subscription, options).instrument(span))
}

async fn handle_socket(
    mut socket: WebSocket,
    bus: EventBus,
    subscription: Subscription,
    options: DeliveryOptions,
) {
    let mut rx = bus.subscribe();
    let (_guard, mut shutdown_rx) =
        ConnectionGuard::subscribe_with_metrics(bus.ws_shutdown_token(), bus.metrics.clone());

    let mut session = SocketSession {
        socket: &mut socket,
        subscription,
        options,
        metrics: bus.metrics.clone(),
        pending: Vec::new(),
        pending_bytes: 0,
    };
    session.run(&mut rx, &mut shutdown_rx).await;

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

/// Per-connection delivery state.
struct SocketSession<'a> {
    socket: &'a mut WebSocket,
    subscription: Subscription,
    options: DeliveryOptions,
    metrics: Option<SharedMetrics>,
    /// Events buffered for the next batch frame.
    pending: Vec<Arc<BusEvent>>,
    /// Payload bytes in `pending`, to keep batches under [`MAX_MESSAGE_SIZE`].
    pending_bytes: usize,
}

impl SocketSession<'_> {
    async fn run(
        &mut self,
        rx: &mut broadcast::Receiver<Arc<BusEvent>>,
        shutdown_rx: &mut broadcast::Receiver<()>,
    ) {
        let mut rate_limiter = WsRateLimiter::new(RATE_LIMIT_WS_BURST, RATE_LIMIT_WS_PERIOD_SECS);
        // Deadline for flushing `pending`; only armed while it is non-empty.
        let mut flush_at: Option<tokio::time::Instant> = None;

        loop {
            let flush_timer = async {
                match flush_at {
                    Some(deadline) => tokio::time::sleep_until(deadline).await,
                    None => std::future::pending().await,
                }
            };

            tokio::select! {
                // Server-initiated graceful shutdown: flush what is buffered,
                // then send a Close frame so the client sees a clean
                // disconnect instead of the TCP connection simply dropping.
                _ = shutdown_rx.recv() => {
                    tracing::info!("server is shutting down; closing websocket connection");
                    let _ = self.flush().await;
                    let _ = self.socket.send(Message::Close(None)).await;
                    break;
                }
                _ = flush_timer => {
                    flush_at = None;
                    if self.flush().await.is_err() {
                        break;
                    }
                }
                result = rx.recv() => {
                    let event = match result {
                        Ok(event) => event,
                        Err(broadcast::error::RecvError::Lagged(missed)) => {
                            tracing::warn!(missed, "websocket client lagged behind event bus");
                            if let Some(metrics) = &self.metrics {
                                metrics.record_ws_dropped("lagged", missed);
                            }
                            let notice = serde_json::json!({ "type": "lagged", "missed": missed });
                            if self.send_frame("control", notice.to_string()).await.is_err() {
                                break;
                            }
                            continue;
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    };
                    if self.accept(event).await.is_err() {
                        break;
                    }
                    // Drain whatever else is already queued so a burst is
                    // packed into as few frames as possible.
                    if self.options.batch_window.is_some() {
                        let mut drained = 0;
                        while drained < self.options.batch_max {
                            match rx.try_recv() {
                                Ok(event) => {
                                    drained += 1;
                                    if self.accept(event).await.is_err() {
                                        return;
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                        if self.pending.is_empty() {
                            flush_at = None;
                        } else if flush_at.is_none() {
                            flush_at = self
                                .options
                                .batch_window
                                .map(|window| tokio::time::Instant::now() + window);
                        }
                    }
                }
                msg = self.socket.recv() => {
                    let msg = match msg {
                        Some(Ok(m)) => m,
                        _ => break,
                    };

                    let reply = match &msg {
                        Message::Text(text) => {
                            if text.len() > MAX_MESSAGE_SIZE {
                                tracing::warn!(
                                    message_size = text.len(),
                                    max_size = MAX_MESSAGE_SIZE,
                                    "Incoming WebSocket text message exceeds size limit, closing connection"
                                );
                                let _ = self.socket.send(Message::Close(None)).await;
                                break;
                            }
                            Some(text.clone())
                        }
                        Message::Binary(data) => {
                            if data.len() > MAX_MESSAGE_SIZE {
                                tracing::warn!(
                                    message_size = data.len(),
                                    max_size = MAX_MESSAGE_SIZE,
                                    "Incoming WebSocket binary message exceeds size limit, closing connection"
                                );
                                let _ = self.socket.send(Message::Close(None)).await;
                                break;
                            }
                            None
                        }
                        Message::Ping(payload) => {
                            let _ = self.socket.send(Message::Pong(payload.clone())).await;
                            continue;
                        }
                        Message::Pong(_) => continue,
                        Message::Close(_) => break,
                    };

                    if !rate_limiter.check() {
                        tracing::warn!("WebSocket connection rate limited");
                        let _ = self.socket.send(Message::Close(None)).await;
                        break;
                    }

                    if let Some(text) = reply {
                        let response = handle_client_command(&mut self.subscription, &text);
                        tracing::debug!(
                            pools = self.subscription.pools.len(),
                            "websocket subscription updated"
                        );
                        if self.send_frame("control", response).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Filter one event and either send it now or add it to the batch.
    async fn accept(&mut self, event: Arc<BusEvent>) -> Result<(), axum::Error> {
        if !self.subscription.matches(&event) {
            return Ok(());
        }
        if event.payload.len() > MAX_MESSAGE_SIZE {
            tracing::warn!(
                message_size = event.payload.len(),
                max_size = MAX_MESSAGE_SIZE,
                "WebSocket message exceeds size limit, dropping"
            );
            if let Some(metrics) = &self.metrics {
                metrics.record_ws_dropped("oversize", 1);
            }
            return Ok(());
        }

        if self.options.batch_window.is_none() {
            return self.send_frame("event", event.payload.to_string()).await;
        }

        if self.pending_bytes + event.payload.len() > MAX_MESSAGE_SIZE {
            self.flush().await?;
        }
        self.pending_bytes += event.payload.len();
        self.pending.push(event);
        if self.pending.len() >= self.options.batch_max {
            self.flush().await?;
        }
        Ok(())
    }

    /// Send every buffered event as a single batch frame.
    async fn flush(&mut self) -> Result<(), axum::Error> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let events = std::mem::take(&mut self.pending);
        self.pending_bytes = 0;
        if let Some(metrics) = &self.metrics {
            metrics.ws_batch_size.observe(events.len() as f64);
        }
        self.send_frame("batch", batch_frame(&events)).await
    }

    async fn send_frame(&mut self, kind: &str, text: String) -> Result<(), axum::Error> {
        let (message, encoding) = encode_frame(text, self.options.compress);
        if let Some(metrics) = &self.metrics {
            let bytes = match &message {
                Message::Text(t) => t.len(),
                Message::Binary(b) => b.len(),
                _ => 0,
            };
            metrics.record_ws_frame(kind, encoding, bytes);
        }
        self.socket.send(message).await
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
        let token = WsShutdownToken::new(1);
        assert_eq!(token.active_count(), 0);

        {
            let (_guard, _shutdown_rx) = ConnectionGuard::subscribe(token.clone());
            assert_eq!(token.active_count(), 1);
        }

        assert_eq!(token.active_count(), 0);
    }

    #[tokio::test]
    async fn connection_guard_shutdown_signal_reaches_subscriber() {
        let token = WsShutdownToken::new(1);
        let (_guard, mut shutdown_rx) = ConnectionGuard::subscribe(token.clone());

        let notified = token.broadcast_shutdown();
        assert_eq!(notified, 1, "the connection's receiver should be notified");
        assert!(
            shutdown_rx.try_recv().is_ok(),
            "connection should observe the shutdown signal"
        );
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

    fn bus_event(json: serde_json::Value) -> BusEvent {
        BusEvent::from_serialize(&json).expect("serialisable")
    }

    fn params() -> WsConnectParams {
        WsConnectParams::default()
    }

    #[test]
    fn bus_event_extracts_routing_keys_once() {
        let event = bus_event(serde_json::json!({
            "type": "prediction_placed",
            "pool_id": 7,
            "user_address": "GABC",
            "amount": 10,
        }));
        assert_eq!(event.event_type.as_deref(), Some("prediction_placed"));
        assert_eq!(event.pool_id, Some(7));
        assert_eq!(event.user_address.as_deref(), Some("GABC"));
        let reparsed: serde_json::Value = serde_json::from_str(&event.payload).unwrap();
        assert_eq!(reparsed["amount"], 10);
    }

    #[test]
    fn empty_subscription_matches_everything() {
        let sub = Subscription::default();
        assert!(sub.matches(&bus_event(serde_json::json!({"pool_id": 1}))));
        assert!(sub.matches(&bus_event(serde_json::json!({"type": "x"}))));
    }

    #[test]
    fn subscription_filters_by_pool_set() {
        let mut sub = Subscription::default();
        sub.add_pools(&[1, 2]).unwrap();
        assert!(sub.matches(&bus_event(serde_json::json!({"pool_id": 2}))));
        assert!(!sub.matches(&bus_event(serde_json::json!({"pool_id": 3}))));
        assert!(
            !sub.matches(&bus_event(serde_json::json!({"type": "no_pool"}))),
            "pool-scoped subscribers must not receive pool-less events"
        );
    }

    #[test]
    fn subscription_filters_by_wallet_and_event_type() {
        let sub = Subscription {
            wallet: Some("GABC".into()),
            event_types: ["prediction_placed".to_string()].into_iter().collect(),
            ..Default::default()
        };
        let matching = serde_json::json!({"type": "prediction_placed", "user_address": "GABC"});
        let wrong_wallet = serde_json::json!({"type": "prediction_placed", "user_address": "GXYZ"});
        let wrong_type = serde_json::json!({"type": "pool_resolved", "user_address": "GABC"});
        assert!(sub.matches(&bus_event(matching)));
        assert!(!sub.matches(&bus_event(wrong_wallet)));
        assert!(!sub.matches(&bus_event(wrong_type)));
    }

    #[test]
    fn subscription_caps_pool_count() {
        let mut sub = Subscription::default();
        let ids: Vec<u64> = (0..MAX_POOL_SUBSCRIPTIONS as u64).collect();
        sub.add_pools(&ids).unwrap();
        // Re-adding an existing pool is fine; a new one is not.
        sub.add_pools(&[0]).unwrap();
        assert!(sub.add_pools(&[u64::MAX]).is_err());
        assert_eq!(sub.pools.len(), MAX_POOL_SUBSCRIPTIONS);
    }

    #[test]
    fn connection_settings_merge_legacy_and_list_pool_params() {
        let p = WsConnectParams {
            pool_id: Some(1),
            pool_ids: Some("2, 3,,".into()),
            events: Some("prediction_placed".into()),
            batch_ms: Some(50),
            batch_max: Some(10_000),
            compress: Some("deflate".into()),
            ..params()
        };
        let (sub, opts) = connection_settings(&p).unwrap();
        assert_eq!(sub.pools.into_iter().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert!(sub.event_types.contains("prediction_placed"));
        assert_eq!(opts.batch_window, Some(Duration::from_millis(50)));
        assert_eq!(opts.batch_max, MAX_BATCH_MAX, "batch_max is clamped");
        assert!(opts.compress);
    }

    #[test]
    fn connection_settings_defaults_preserve_legacy_behaviour() {
        let (sub, opts) = connection_settings(&params()).unwrap();
        assert_eq!(sub, Subscription::default());
        assert_eq!(opts, DeliveryOptions::default());
    }

    #[test]
    fn connection_settings_reject_bad_input() {
        let bad_pool = WsConnectParams {
            pool_ids: Some("1,abc".into()),
            ..params()
        };
        assert!(connection_settings(&bad_pool).is_err());
        let bad_batch = WsConnectParams {
            batch_ms: Some(5_000),
            ..params()
        };
        assert!(connection_settings(&bad_batch).is_err());
        let bad_codec = WsConnectParams {
            compress: Some("brotli".into()),
            ..params()
        };
        assert!(connection_settings(&bad_codec).is_err());
    }

    #[test]
    fn client_commands_update_subscription() {
        let mut sub = Subscription::default();
        let reply = handle_client_command(&mut sub, r#"{"action":"subscribe","pool_ids":[5,6]}"#);
        let reply: serde_json::Value = serde_json::from_str(&reply).unwrap();
        assert_eq!(reply["type"], "subscriptions");
        assert_eq!(reply["pool_ids"], serde_json::json!([5, 6]));

        handle_client_command(&mut sub, r#"{"action":"unsubscribe","pool_ids":[5]}"#);
        assert_eq!(sub.pools.iter().copied().collect::<Vec<_>>(), vec![6]);

        let reply = handle_client_command(&mut sub, r#"{"action":"list"}"#);
        assert!(reply.contains("\"pool_ids\":[6]"));

        let reply = handle_client_command(&mut sub, "not json");
        assert!(reply.contains("\"type\":\"error\""));
    }

    #[test]
    fn batch_frame_is_valid_json_with_raw_payloads() {
        let events: Vec<Arc<BusEvent>> = (1..=3)
            .map(|i| Arc::new(bus_event(serde_json::json!({"pool_id": i}))))
            .collect();
        let frame: serde_json::Value = serde_json::from_str(&batch_frame(&events)).unwrap();
        assert_eq!(frame["type"], "batch");
        assert_eq!(frame["count"], 3);
        assert_eq!(frame["events"][2]["pool_id"], 3);
    }

    #[test]
    fn encode_frame_compresses_only_large_frames_when_enabled() {
        let small = "{\"a\":1}".to_string();
        assert!(matches!(
            encode_frame(small.clone(), true).0,
            Message::Text(_)
        ));

        let large = format!("[{}]", vec!["{\"pool_id\":1}"; 200].join(","));
        assert!(large.len() >= COMPRESSION_THRESHOLD);
        assert!(matches!(
            encode_frame(large.clone(), false).0,
            Message::Text(_)
        ));

        let (message, encoding) = encode_frame(large.clone(), true);
        assert_eq!(encoding, "deflate");
        let Message::Binary(bytes) = message else {
            panic!("large frame should be binary when compressed");
        };
        assert!(bytes.len() < large.len());

        use std::io::Read as _;
        let mut inflated = String::new();
        flate2::read::ZlibDecoder::new(bytes.as_slice())
            .read_to_string(&mut inflated)
            .unwrap();
        assert_eq!(inflated, large);
    }

    #[tokio::test]
    async fn event_bus_delivers_shared_events_to_every_subscriber() {
        let bus = EventBus::new();
        let mut a = bus.subscribe();
        let mut b = bus.subscribe();
        bus.send(&serde_json::json!({"type": "prediction_placed", "pool_id": 9}));
        let ea = a.recv().await.unwrap();
        let eb = b.recv().await.unwrap();
        assert!(
            Arc::ptr_eq(&ea, &eb),
            "payload is serialised once and shared"
        );
        assert_eq!(ea.pool_id, Some(9));
    }

    #[test]
    fn connection_guard_updates_ws_metrics() {
        let metrics = Arc::new(crate::metrics::Metrics::new().unwrap());
        let token = WsShutdownToken::new(1);
        {
            let (_guard, _rx) =
                ConnectionGuard::subscribe_with_metrics(token.clone(), Some(metrics.clone()));
            assert_eq!(metrics.ws_connections_active.get(), 1.0);
        }
        assert_eq!(metrics.ws_connections_active.get(), 0.0);
        assert_eq!(metrics.ws_disconnections_total.get(), 1.0);
    }

    #[test]
    fn client_ip_prefers_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "10.0.0.1, 10.0.0.2".parse().unwrap());
        headers.insert("x-real-ip", "9.9.9.9".parse().unwrap());
        assert_eq!(client_ip(&headers), "10.0.0.1");
    }
}
