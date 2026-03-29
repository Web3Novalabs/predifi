use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::config::Config;

/// Create a PostgreSQL connection pool using conservative defaults.
///
/// This uses lazy connection mode so local development can start the server
/// without requiring an active database until a query is executed.
pub fn create_pool(config: &Config) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .min_connections(config.db_min_connections)
        .acquire_timeout(Duration::from_secs(config.db_acquire_timeout_secs))
        .connect_lazy(&config.database_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn creates_pool_from_valid_config() {
        let config = Config {
            host: String::from("0.0.0.0"),
            port: 3000,
            database_url: String::from("postgres://postgres:postgres@localhost:5432/predifi"),
            db_max_connections: 5,
            db_min_connections: 1,
            db_acquire_timeout_secs: 5,
            log_level: String::from("info"),
        };

        let pool = create_pool(&config).expect("pool should initialize in lazy mode");
        assert!(!pool.is_closed(), "new pool should start open");
    }
}
