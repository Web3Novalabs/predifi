use crate::config::db_config::DbConfig;

use sqlx::{Pool, Postgres};
use std::time::Instant;
use tracing::Instrument;

#[derive(Clone)]
pub struct Database {
    pool: Pool<Postgres>,
}

impl Database {
    /// Expose a reference to the connection pool for migrations and queries
    pub fn pool(&self) -> &Pool<Postgres> {
        &self.pool
    }
}

impl Database {
    pub async fn connect(config: &DbConfig) -> Self {
        let connect_span = tracing::info_span!(
            "database_connect",
            database.type = "postgresql",
            database.max_connections = config.max_connections,
        );

        let pool = async {
            tracing::info!(
                event = "database_connection_start",
                database.max_connections = config.max_connections,
                "Starting database connection"
            );

            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(config.max_connections)
                .connect(&config.url)
                .await
                .expect("Failed to connect to Postgres");

            tracing::info!(
                event = "database_connection_pool_created",
                database.max_connections = config.max_connections,
                "Database connection pool created successfully"
            );

            pool
        }
        .instrument(connect_span)
        .await;

        Self { pool }
    }

    pub async fn ping(&self) -> Result<i32, sqlx::Error> {
        let ping_span = tracing::info_span!(
            "database_ping",
            database.operation = "ping",
            database.query = "SELECT 1",
        );

        async {
            let start_time = Instant::now();

            tracing::debug!(
                event = "database_query_start",
                database.operation = "ping",
                database.query = "SELECT 1",
                "Starting database ping query"
            );

            let result: Result<(i32,), sqlx::Error> =
                sqlx::query_as("SELECT 1").fetch_one(&self.pool).await;

            let duration = start_time.elapsed();

            match result {
                Ok((value,)) => {
                    tracing::info!(
                        event = "database_query_success",
                        database.operation = "ping",
                        database.query = "SELECT 1",
                        database.result = value,
                        database.duration_ms = duration.as_millis().to_string(),
                        "Database ping successful"
                    );
                    Ok(value)
                }
                Err(e) => {
                    tracing::error!(
                        event = "database_query_error",
                        database.operation = "ping",
                        database.query = "SELECT 1",
                        database.duration_ms = duration.as_millis().to_string(),
                        error = %e,
                        "Database ping failed"
                    );
                    Err(e)
                }
            }
        }
        .instrument(ping_span)
        .await
    }
}
