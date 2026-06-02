//! Integration tests for the database layer using testcontainers-rs.
//!
//! Each test spins up a throwaway Postgres container via the shared fixture
//! module, runs all migrations, and exercises the real SQL queries — no
//! pre-configured database required.
//!
//! # Resource lifecycle
//!
//! `setup_postgres()` returns both the connection pool **and** the container
//! handle.
//! Tests must bind the container to a named variable (not `_`) so that it
//! stays alive for the entire test body.  At the end of each test the
//! container is dropped, which stops the Docker container and releases the
//! ephemeral port.  The pool is closed explicitly with `pool.close().await`
//! before the container is dropped so that all in-flight connections are
//! cleanly terminated first.

#[cfg(test)]
mod tests {
    use sqlx::Row;

    use crate::test_support;

    #[tokio::test]
    async fn migrations_run_cleanly() {
        let (pool, container) = test_support::setup_postgres().await;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

        pool.close().await;
        drop(container);
    }

    #[tokio::test]
    async fn insert_and_query_pool() {
        let (pool, container) = test_support::setup_postgres().await;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

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
    async fn insert_and_query_prediction() {
        let (pool, container) = test_support::setup_postgres().await;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

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

    #[tokio::test]
    async fn pool_status_defaults_to_open() {
        let (pool, container) = test_support::setup_postgres().await;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

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
