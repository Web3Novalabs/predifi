//! Lightweight test helpers used by the unit test suite.
//!
//! This module provides a mock Stellar RPC server and a `setup_healthy_test_env`
//! helper without requiring `testcontainers` or Docker.

use std::collections::HashMap;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    task::JoinHandle,
};

use crate::config::Config;
use crate::price_cache::PriceCache;

// ── MockRpcServer ─────────────────────────────────────────────────────────────

/// A running mock Stellar RPC server for unit tests.
///
/// Binds an ephemeral port and responds to every HTTP request with a minimal
/// `{"result":{"status":"healthy"}}` payload so health-check probes succeed
/// without hitting the real Stellar network.
pub struct MockRpcServer {
    url: String,
    shutdown_tx: oneshot::Sender<()>,
    handle: JoinHandle<()>,
}

impl MockRpcServer {
    /// Bind an ephemeral port and start accepting connections in the background.
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind mock RPC listener");
        let addr = listener.local_addr().expect("failed to get local addr");
        let url = format!("http://{addr}");

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    result = listener.accept() => {
                        if let Ok((mut socket, _)) = result {
                            tokio::spawn(async move {
                                let mut buf = [0u8; 1024];
                                let _ = socket.read(&mut buf).await;
                                let body =
                                    r#"{"jsonrpc":"2.0","id":1,"result":{"status":"healthy"}}"#;
                                let response = format!(
                                    "HTTP/1.1 200 OK\r\n\
                                     Content-Type: application/json\r\n\
                                     Content-Length: {}\r\n\
                                     Connection: close\r\n\
                                     \r\n\
                                     {}",
                                    body.len(),
                                    body
                                );
                                let _ = socket.write_all(response.as_bytes()).await;
                                let _ = socket.shutdown().await;
                            });
                        }
                    }
                }
            }
        });

        Self { url, shutdown_tx, handle }
    }

    /// Base URL to assign to [`Config::stellar_rpc_url`].
    pub fn url(&self) -> String {
        self.url.clone()
    }

    /// Stop the accept loop and wait for the background task to exit.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        let _ = self.handle.await;
    }
}

// ── Test environment builder ──────────────────────────────────────────────────

/// Default asset prices that satisfy health-check readiness in tests.
pub fn default_test_prices() -> HashMap<String, f64> {
    HashMap::from([
        ("BTC".to_string(), 60_000.0),
        ("ETH".to_string(), 3_000.0),
        ("XLM".to_string(), 0.12),
    ])
}

/// Start a mock Stellar RPC server and return a test [`Config`] + populated cache.
///
/// Call [`MockRpcServer::shutdown`] when the test finishes so the ephemeral
/// port is released before the next test runs.
pub async fn setup_healthy_test_env() -> (Config, PriceCache, MockRpcServer) {
    let mock = MockRpcServer::start().await;
    let mut config = Config::default_for_test();
    config.stellar_rpc_url = mock.url();

    let cache = PriceCache::new();
    cache.update(default_test_prices());

    (config, cache, mock)
}
