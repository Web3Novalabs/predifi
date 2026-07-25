//! Mock Stellar RPC server for integration tests.
//!
//! Binds an ephemeral port and responds to every HTTP request with a minimal
//! JSON-RPC 2.0 payload so health probes succeed without hitting the real network.

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    task::JoinHandle,
};

/// A running mock RPC server that can be cleanly shut down after a test.
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
        let url = format!("http://{}", addr);

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

        Self {
            url,
            shutdown_tx,
            handle,
        }
    }

    /// Base URL to assign to [`crate::config::Config::stellar_rpc_url`].
    pub fn url(&self) -> String {
        self.url.clone()
    }

    /// Stop the accept loop and wait for the background task to exit.
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        let _ = self.handle.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_rpc_responds_to_health_probe() {
        let mock = MockRpcServer::start().await;
        let client = reqwest::Client::new();
        let response = client
            .post(mock.url())
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getHealth"
            }))
            .send()
            .await
            .expect("request should succeed");

        assert!(response.status().is_success());
        mock.shutdown().await;
    }
}
