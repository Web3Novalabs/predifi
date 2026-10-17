use predifi_backend::{config::Config, run_server};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config::from_env().unwrap_or_else(|error| {
        eprintln!("failed to load configuration: {error}");
        std::process::exit(1);
    });

    // `run_server` installs the JSON tracing subscriber (per-module
    // `RUST_LOG` filter, optional OTel export) before starting the server.
    run_server(config).await;
}
