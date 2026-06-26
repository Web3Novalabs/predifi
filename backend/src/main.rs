//! `predifi-backend` — Axum HTTP server entry point.
//!
//! All routers, handlers, and shared modules live in the `predifi_backend`
//! library crate so they can be reused by other binaries (notably
//! `predifi-seed`).  This file only wires environment loading to
//! [`predifi_backend::run_server`].

use predifi_backend::{config::Config, run_server};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config::from_env().unwrap_or_else(|error| {
        eprintln!("failed to load configuration: {error}");
        std::process::exit(1);
    });

    run_server(config).await;
}
