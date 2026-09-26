//! Integration tests for the database layer using testcontainers-rs.
//!
//! Each test spins up a throwaway Postgres container, runs all migrations,
//! and exercises the real SQL queries — no pre-configured database required.
//!
//! # Resource lifecycle
//!
//! `setup()` returns both the connection pool **and** the container handle.
//! Tests must bind the container to a named variable (not `_`) so that it
//! stays alive for the entire test body.  At the end of each test the
//! container is dropped, which stops the Docker container and releases the
//! ephemeral port.  The pool is closed explicitly with `pool.close().await`
//! before the container is dropped so that all in-flight connections are
//! cleanly terminated first.

#[cfg(test)]
mod tests {
    use sqlx::{postgres::PgPoolOptions, PgPool, Row};
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::postgres::Postgres;

    /// Boot a Postgres container, run all migrations, and return the pool
    /// together with the container handle.
    ///
    /// # Important
    ///
    /// The caller **must** keep the returned `ContainerAsync` bound to a named
    /// variable for the full duration of the test.  Binding it to `_` would
    /// drop it immediately, stopping the container before any queries run.
    ///
    /// Always close the pool before dropping the container:
    ///
    /// ```rust,ignore
    /// let (pool, container) = setup().await;
    /// // … run queries …
    /// pool.close().await;
    /// drop(container); // container stops here
    /// ```
    async fn setup() -> (PgPool, testcontainers::ContainerAsync<Postgres>) {
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
            .max_connections(5)
            .connect(&url)
            .await
            .expect("connect to postgres");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

        (pool, container)
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn migrations_run_cleanly() {
        let (pool, container) = setup().await;
        // If setup() completes without panic the migrations are valid.
        pool.close().await;
        drop(container);
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn insert_and_query_pool() {
        let (pool, container) = setup().await;

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

        pool.close().await;
        drop(container);
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn insert_and_query_prediction() {
        let (pool, container) = setup().await;

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

        let count: i64 = sqlx::query("SELECT COUNT(*) FROM predictions WHERE user_address = $1")
            .bind("GABC1234")
            .fetch_one(&pool)
            .await
            .expect("count predictions")
            .get(0);

        assert_eq!(count, 1);

        pool.close().await;
        drop(container);
    }

    /// The pool-scoped leaderboard query (`/pools/:id/leaderboard`) must use
    /// an index on `predictions.pool_id`, not a sequential scan, once the
    /// table holds enough rows to make the difference measurable (#1733).
    ///
    /// Seeds many pools with many predictions each so that filtering to one
    /// `pool_id` is highly selective (~0.3%), which is well within the range
    /// where Postgres' planner should prefer `idx_predictions_pool_id` (or
    /// one of the other `pool_id`-prefixed indexes from later migrations)
    /// over scanning the whole table. If that index is ever dropped, this
    /// selectivity gap means the planner falls back to a sequential scan and
    /// the assertion below fails.
    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn leaderboard_query_uses_index_not_seq_scan_on_predictions() {
        let (pool, container) = setup().await;

        const POOL_COUNT: i64 = 300;
        const PREDICTIONS_PER_POOL: i64 = 300;

        sqlx::query(
            r#"
            INSERT INTO pools (pool_id, name, category, total_stake, end_time, state, creator, token)
            SELECT g, 'Pool ' || g, 'Test', 0, NOW() + INTERVAL '1 day', 'active', 'GCREATOR', 'XLM'
            FROM generate_series(1, $1) AS g
            "#,
        )
        .bind(POOL_COUNT)
        .execute(&pool)
        .await
        .expect("seed pools");

        sqlx::query(
            r#"
            INSERT INTO predictions (pool_id, user_address, outcome, amount)
            SELECT g, 'GUSER' || ((g * 100000) + s)::text, (s % 2), 100
            FROM generate_series(1, $1) AS g, generate_series(1, $2) AS s
            "#,
        )
        .bind(POOL_COUNT)
        .bind(PREDICTIONS_PER_POOL)
        .execute(&pool)
        .await
        .expect("seed predictions");

        // Fresh statistics are essential: without them the planner may still
        // guess a sequential scan for a table it thinks is tiny/default-sized.
        sqlx::query("ANALYZE predictions")
            .execute(&pool)
            .await
            .expect("analyze predictions");
        sqlx::query("ANALYZE pools")
            .execute(&pool)
            .await
            .expect("analyze pools");

        let plan = crate::db::explain_leaderboard_plan(&pool, "volume", "all", Some(1), 20, 0)
            .await
            .expect("explain leaderboard query");

        assert!(
            !plan.to_lowercase().contains("seq scan on predictions"),
            "leaderboard query for a single pool_id must use an index on predictions, \
             not a sequential scan; got plan:\n{plan}"
        );

        pool.close().await;
        drop(container);
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn pool_status_defaults_to_open() {
        let (pool, container) = setup().await;

        sqlx::query(
            "INSERT INTO pools (metadata_url, start_time, end_time) \
             VALUES ($1, NOW(), NOW() + INTERVAL '1 day')",
        )
        .bind("https://example.com/pool/3")
        .execute(&pool)
        .await
        .expect("insert pool");

        let status: String = sqlx::query("SELECT status FROM pools ORDER BY id DESC LIMIT 1")
            .fetch_one(&pool)
            .await
            .expect("fetch status")
            .get(0);

        assert_eq!(status, "Open");

        pool.close().await;
        drop(container);
    }
}
