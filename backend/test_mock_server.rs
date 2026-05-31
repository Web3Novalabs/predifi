//! Mock RPC server for use in integration tests.
//!
//! Provides a lightweight TCP server that speaks just enough HTTP to satisfy
//! the Stellar RPC health-check call made by the `/health` endpoint.
//!
//! # Usage
//!
//! ```rust,ignore
//! let mock = MockRpcServer::start().await;
//! config.stellar_rpc_url = mock.url();
//! // … run your test …
//! mock.shutdown().await;
//! ```

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    task::JoinHandle,
};

/// A running mock RPC server that can be cleanly shut down after a test.
pub struct MockRpcServer {
    /// Base URL callers should point their RPC client at.
    url: String,
    /// Sending half of the shutdown channel — dropping or sending signals the
    /// background task to stop accepting new connections.
    shutdown_tx: oneshot::Sender<()>,
    /// Handle to the background accept loop so callers can await full teardown.
    handle: JoinHandle<()>,
}

impl MockRpcServer {
    /// Bind an ephemeral port and start accepting connections in the background.
    ///
    /// The server responds to every request with a minimal JSON-RPC 2.0
    /// `{"result":{"status":"healthy"}}` payload, which is enough to satisfy
    /// the Stellar RPC health probe.
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind mock RPC listener");
        let addr = listener.local_addr().expect("failed to get local addr");
        let url = format!("http://{}", addr);

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Stop the accept loop when the shutdown signal arrives.
                    _ = &mut shutdown_rx => break,

                    result = listener.accept() => {
                        if let Ok((mut socket, _)) = result {
                            tokio::spawn(async move {
                                let mut buf = [0u8; 1024];
                                // Read the request (we don't need to parse it).
                                let _ = socket.read(&mut buf).await;
                                let body = r#"{"jsonrpc":"2.0","id":1,"result":{"status":"healthy"}}"#;
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
                                // Flush and close the socket cleanly.
                                let _ = socket.shutdown().await;
                            });
                        }
                    }
                }
            }
        });

        Self {
            url,
            shutdown_tx,
            handle,
        }
    }

    /// The base URL tests should use as `stellar_rpc_url`.
    pub fn url(&self) -> String {
        self.url.clone()
    }

    /// Signal the accept loop to stop and wait for the background task to exit.
    ///
    /// Call this at the end of every test that uses a [`MockRpcServer`] to
    /// ensure the socket is released before the next test runs.
    pub async fn shutdown(self) {
        // Ignore send errors — the task may have already exited.
        let _ = self.shutdown_tx.send(());
        // Await the task so the port is fully released before we return.
        let _ = self.handle.await;
    }
}
