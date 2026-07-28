//! WebSocket broadcast for live prediction events.
//!
//! Clients connect at `GET /api/v1/ws?address=<wallet>` to receive only events
//! where `user_address` matches the subscribed wallet. Omitting `address` delivers
//! all events (useful for dashboards). The indexer calls [`EventBus::send`] whenever
//! a new prediction is indexed.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::broadcast;
use tracing::info_span;
use tracing::Instrument;

const CHANNEL_CAPACITY: usize = 256;

/// Optional query parameters for the WebSocket endpoint.
#[derive(Debug, Deserialize, Default)]
pub struct WsConnectParams {
    /// When set, only events whose `user_address` equals this value are forwarded.
    pub address: Option<String>,
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

/// Returns `true` when `json` should be delivered to a subscriber with `wallet_filter`.
///
/// When `wallet_filter` is `None`, all well-formed events are delivered.
/// When set, only events containing a matching `user_address` field are delivered.
pub fn should_deliver_event(json: &str, wallet_filter: Option<&str>) -> bool {
    let Some(filter) = wallet_filter else {
        return true;
    };

    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return false;
    };

    value
        .get("user_address")
        .and_then(|v| v.as_str())
        .is_some_and(|addr| addr == filter)
}

/// Axum handler — upgrades the HTTP connection to WebSocket and streams events.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsConnectParams>,
    State(bus): State<EventBus>,
) -> impl IntoResponse {
    let wallet_filter = params.address;
    let span = info_span!("websocket.connect", wallet = ?wallet_filter);
    ws.on_upgrade(move |socket| handle_socket(socket, bus, wallet_filter).instrument(span))
}

async fn handle_socket(mut socket: WebSocket, bus: EventBus, wallet_filter: Option<String>) {
    let mut rx = bus.subscribe();

    let count = bus.active_connections.fetch_add(1, Ordering::Relaxed) + 1;
    tracing::info!(
        active_connections = count,
        wallet = ?wallet_filter,
        "websocket client connected"
    );

    run_socket(&mut socket, &mut rx, wallet_filter.as_deref()).await;

    let count = bus.active_connections.fetch_sub(1, Ordering::Relaxed) - 1;
    tracing::info!(active_connections = count, "websocket client disconnected");
}

async fn run_socket(
    socket: &mut WebSocket,
    rx: &mut broadcast::Receiver<String>,
    wallet_filter: Option<&str>,
) {
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if !should_deliver_event(&msg, wallet_filter) {
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
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::should_deliver_event;

    #[test]
    fn delivers_all_events_when_no_wallet_filter() {
        let json = r#"{"type":"prediction_placed","user_address":"GABC","pool_id":1}"#;
        assert!(should_deliver_event(json, None));
    }

    #[test]
    fn delivers_event_when_wallet_matches() {
        let json = r#"{"type":"prediction_placed","user_address":"GABC","pool_id":1}"#;
        assert!(should_deliver_event(json, Some("GABC")));
    }

    #[test]
    fn skips_event_when_wallet_mismatch() {
        let json = r#"{"type":"prediction_placed","user_address":"GABC","pool_id":1}"#;
        assert!(!should_deliver_event(json, Some("GXYZ")));
    }

    #[test]
    fn skips_malformed_json_when_filter_active() {
        assert!(!should_deliver_event("not-json", Some("GABC")));
    }

    #[test]
    fn skips_event_missing_user_address_when_filter_active() {
        let json = r#"{"type":"prediction_placed","pool_id":1}"#;
        assert!(!should_deliver_event(json, Some("GABC")));
    }
}
