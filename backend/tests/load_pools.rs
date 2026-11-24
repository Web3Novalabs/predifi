//! Load test for the `GET /v1/pools` endpoint.
//!
//! Measures throughput and latency percentiles (p50, p95, p99) under a
//! configurable concurrency level against a real PostgreSQL database.
//!
//! ## Running
//!
//! ```bash
//! cd backend && cargo test --test load_pools -- --nocapture
//! ```
//!
//! Docker is required because the test spins up a throwaway Postgres container
//! via `testcontainers`.  The container is created and destroyed automatically.
//!
//! ## Baseline
//!
//! Record the reported p50 / p95 / p99 values as the baseline.  Future changes
//! that increase these numbers should be investigated as regressions.

use axum::http::Request;
use http_body_util::BodyExt;
use predifi_backend::{
    build_router_with_db,
    config::Config,
    metrics::Metrics,
    price_cache::PriceCache,
    redis_cache::RedisCache,
    ws,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower::ServiceExt;

const CONCURRENT_REQUESTS: usize = 50;
const TOTAL_REQUESTS: usize = 500;

fn build_request() -> Request<axum::body::Body> {
    Request::builder()
        .method("GET")
        .uri("/v1/pools")
        .body(axum::body::Body::empty())
        .expect("failed to build request")
}

async fn body_string(body: axum::body::Body) -> String {
    let bytes = body
        .collect()
        .await
        .expect("failed to collect body")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("body is not valid utf-8")
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let index = ((sorted.len() as f64) * p / 100.0).ceil() as usize - 1;
    let index = index.min(sorted.len() - 1);
    sorted[index]
}

#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn load_test_pools_endpoint_reports_latency_percentiles() {
    let container = testcontainers::runners::AsyncRunner::default()
        .run(testcontainers_modules::postgres::Postgres::default())
        .await
        .expect("postgres container");

    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres port");

    let database_url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("connect to test postgres");

    sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
        .await
        .expect("load migrator")
        .run(&pool)
        .await
        .expect("apply migrations");

    let mut config = Config::default_for_test();
    config.database_url = database_url;
    config.redis_url = "redis://localhost:6379".to_string();

    let metrics = Arc::new(Metrics::new().expect("metrics"));

    let router = build_router_with_db(
        config,
        PriceCache::new(),
        RedisCache::disabled(),
        pool,
        ws::EventBus::new(),
    );

    let mut latencies: Vec<Duration> = Vec::with_capacity(TOTAL_REQUESTS);
    let mut errors = 0usize;
    let start = Instant::now();

    let semaphore = Arc::new(tokio::sync::Semaphore::new(CONCURRENT_REQUESTS));
    let mut handles = Vec::with_capacity(TOTAL_REQUESTS);

    for _ in 0..TOTAL_REQUESTS {
        let permit = semaphore.clone().acquire_owned().await.expect("semaphore");
        let router = router.clone();
        let handle = tokio::spawn(async move {
            let _permit = permit;
            let req_start = Instant::now();
            let result = router.oneshot(build_request()).await;
            let elapsed = req_start.elapsed();
            (result, elapsed)
        });
        handles.push(handle);
    }

    for handle in handles {
        let (result, elapsed) = handle.await.expect("task panicked");
        latencies.push(elapsed);
        if let Ok(response) = result {
            if !response.status().is_success() {
                errors += 1;
            }
        } else {
            errors += 1;
        }
    }

    let total_duration = start.elapsed();
    latencies.sort_unstable();

    let p50 = percentile(&latencies, 50.0);
    let p95 = percentile(&latencies, 95.0);
    let p99 = percentile(&latencies, 99.0);
    let throughput = TOTAL_REQUESTS as f64 / total_duration.as_secs_f64();

    eprintln!("=== Pool listing load test ===");
    eprintln!("concurrency:     {}", CONCURRENT_REQUESTS);
    eprintln!("total_requests:  {}", TOTAL_REQUESTS);
    eprintln!("errors:          {errors}");
    eprintln!("duration:        {:?}", total_duration);
    eprintln!("throughput:      {throughput:.2} req/s");
    eprintln!("p50_latency:     {:?}", p50);
    eprintln!("p95_latency:     {:?}", p95);
    eprintln!("p99_latency:     {:?}", p99);

    assert!(
        errors < TOTAL_REQUESTS / 2,
        "more than 50% of requests failed: {errors} / {TOTAL_REQUESTS}"
    );
}
