//! WebSocket broadcast for live prediction events.
//!
//! A single `tokio::sync::broadcast` channel fans out JSON payloads to every
//! connected client. The indexer calls [`EventBus::send`] whenever a new
//! prediction is indexed; the WS handler at `GET /ws` forwards each message
//! to the client until the connection closes.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use serde::Serialize;
use tokio::sync::broadcast;
use tracing::info_span;
use tracing::Instrument;

const CHANNEL_CAPACITY: usize = 256;

/// Shareable handle to the broadcast channel.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<String>,
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
        Self { tx }
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

/// Axum handler — upgrades the HTTP connection to WebSocket and streams events.
pub async fn ws_handler(ws: WebSocketUpgrade, State(bus): State<EventBus>) -> impl IntoResponse {
    let span = info_span!("websocket.connect");
    ws.on_upgrade(move |socket| handle_socket(socket, bus).instrument(span))
}

async fn handle_socket(mut socket: WebSocket, bus: EventBus) {
    let mut rx = bus.subscribe();
    loop {
        tokio::select! {
            // Forward broadcast messages to the client.
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if socket.send(Message::Text(msg)).await.is_err() {
                            break; // client disconnected
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            // Stop if the client closes the connection.
            msg = socket.recv() => {
                if msg.is_none() { break; }
            }
        }
    }
}
