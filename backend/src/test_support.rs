//! Shared test fixtures for integration tests.
//!
//! Keeping container bootstrapping in one place avoids duplicating the setup
//! logic across the Postgres and Redis integration suites.

use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::{postgres::Postgres, redis::Redis};

use crate::redis_cache::RedisCache;

/// Start a temporary Postgres container and return a SQLx pool bound to it.
///
/// Callers should keep the returned container alive for the duration of the
/// test and close the pool before dropping the container.
#[allow(dead_code)]
pub async fn setup_postgres() -> (PgPool, testcontainers::ContainerAsync<Postgres>) {
    let container = Postgres::default()
        .start()
        .await
        .expect("postgres container");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres port");
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect to test postgres");

    (pool, container)
}

/// Start a temporary Redis container and return a cache client bound to it.
///
/// A short delay gives the connection manager time to fully initialize before
/// the tests begin issuing commands.
#[allow(dead_code)]
pub async fn setup_redis() -> (RedisCache, testcontainers::ContainerAsync<Redis>) {
    let container = Redis::default().start().await.expect("redis container");
    let port = container
        .get_host_port_ipv4(6379)
        .await
        .expect("redis port");
    let url = format!("redis://127.0.0.1:{port}");

    let cache = RedisCache::new(&url).await;

    tokio::time::sleep(Duration::from_millis(200)).await;

    (cache, container)
}
