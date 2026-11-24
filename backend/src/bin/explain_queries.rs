//! Print `EXPLAIN (ANALYZE, BUFFERS)` plans for the API's critical read
//! queries (#2204). Read-only: every statement runs in a rolled-back
//! transaction. See `predifi_backend::db::explain`.
//!
//! Usage: `predifi-explain-queries [USER_ADDRESS] [POOL_ID]`
//!
//! When no address / pool id is given, the busiest user and the largest pool
//! in the database are used so the plans reflect realistic row counts.

use std::process::ExitCode;

use predifi_backend::{
    config::Config,
    db::{
        create_pool,
        explain::{critical_queries, explain_analyze},
    },
};

#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::dotenv().ok();

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("failed to load configuration: {error}");
            return ExitCode::FAILURE;
        }
    };

    let pool = match create_pool(&config).await {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("failed to connect to database: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut args = std::env::args().skip(1);
    let address = match args.next() {
        Some(address) => address,
        None => sqlx::query_scalar::<_, String>(
            "SELECT user_address FROM predictions \
             GROUP BY user_address ORDER BY COUNT(*) DESC LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_default(),
    };
    let pool_id = match args.next().and_then(|v| v.parse::<i64>().ok()) {
        Some(pool_id) => pool_id,
        None => sqlx::query_scalar::<_, i64>(
            "SELECT pool_id FROM pools ORDER BY total_stake DESC LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(1),
    };

    println!("-- user_address = {address:?}, pool_id = {pool_id}\n");

    let mut failed = false;
    for query in critical_queries(&address, pool_id) {
        println!("=== {} ===", query.name);
        println!("endpoint: {}   source: {}", query.endpoint, query.source);
        match explain_analyze(&pool, &query.sql, &query.params).await {
            Ok(plan) => println!("{plan}\n"),
            Err(error) => {
                failed = true;
                println!("EXPLAIN failed: {error}\n");
            }
        }
    }

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
