use crate::config::db_config::DbConfig;
use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    pub async fn connect(config: &DbConfig) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .connect(&config.url)
            .await
            .expect("Failed to create Postgres connection pool");
        Self { pool }
    }

    pub async fn ping(&self) -> Result<i32, sqlx::Error> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
    }
}
