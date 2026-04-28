//! Integration tests for the database layer using testcontainers-rs.
//!
//! Each test spins up a throwaway Postgres container, runs all migrations,
//! and exercises the real SQL queries — no pre-configured database required.

#[cfg(test)]
mod tests {
    use sqlx::{postgres::PgPoolOptions, PgPool, Row};
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::postgres::Postgres;

    /// Boot a Postgres container, run all migrations, and return the pool.
    async fn setup() -> (PgPool, testcontainers::ContainerAsync<Postgres>) {
        let container = Postgres::default().start().await.expect("postgres container");
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

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

        (pool, container)
    }

    #[tokio::test]
    async fn migrations_run_cleanly() {
        let (_pool, _container) = setup().await;
        // If setup() completes without panic the migrations are valid.
    }

    #[tokio::test]
    async fn insert_and_query_pool() {
        let (pool, _container) = setup().await;

        sqlx::query(
            "INSERT INTO pools (metadata_url, start_time, end_time) \
             VALUES ($1, NOW(), NOW() + INTERVAL '1 day')",
        )
        .bind("https://example.com/pool/1")
        .execute(&pool)
        .await
        .expect("insert pool");

        let count: i64 = sqlx::query("SELECT COUNT(*) FROM pools")
            .fetch_one(&pool)
            .await
            .expect("count pools")
            .get(0);

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn insert_and_query_prediction() {
        let (pool, _container) = setup().await;

        let pool_id: i64 = sqlx::query(
            "INSERT INTO pools (metadata_url, start_time, end_time) \
             VALUES ($1, NOW(), NOW() + INTERVAL '1 day') RETURNING id",
        )
        .bind("https://example.com/pool/2")
        .fetch_one(&pool)
        .await
        .expect("insert pool")
        .get(0);

        sqlx::query(
            "INSERT INTO predictions (pool_id, user_address, outcome, amount) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(pool_id)
        .bind("GABC1234")
        .bind(1_i32)
        .bind(500_i64)
        .execute(&pool)
        .await
        .expect("insert prediction");

        let count: i64 =
            sqlx::query("SELECT COUNT(*) FROM predictions WHERE user_address = $1")
                .bind("GABC1234")
                .fetch_one(&pool)
                .await
                .expect("count predictions")
                .get(0);

        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn pool_status_defaults_to_open() {
        let (pool, _container) = setup().await;

        sqlx::query(
            "INSERT INTO pools (metadata_url, start_time, end_time) \
             VALUES ($1, NOW(), NOW() + INTERVAL '1 day')",
        )
        .bind("https://example.com/pool/3")
        .execute(&pool)
        .await
        .expect("insert pool");

        let status: String =
            sqlx::query("SELECT status FROM pools ORDER BY id DESC LIMIT 1")
                .fetch_one(&pool)
                .await
                .expect("fetch status")
                .get(0);

        assert_eq!(status, "Open");
    }
}
