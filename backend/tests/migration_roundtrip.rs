//! Integration tests to verify database migrations can be applied and rolled back.
//!
//! These tests spin up a throwaway Postgres container, run the repository migrations,
//! verify that schema objects are created, then revert the migrations and verify
//! that schema objects are removed. They are intended as a smoke test for the
//! SQL migration scripts in `backend/migrations`.

#[cfg(test)]
mod tests {
    use sqlx::{postgres::PgPoolOptions, PgPool};
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::postgres::Postgres;

    async fn start_postgres() -> (PgPool, testcontainers::ContainerAsync<Postgres>) {
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

    #[tokio::test]
    async fn migrations_apply_and_revert() {
        let (pool, _container) = start_postgres().await;

        // Load migrations from the backend/migrations directory at runtime.
        let migrator = sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .expect("load migrator");
        migrator.run(&pool).await.expect("apply migrations");

        // Confirm a known table exists (pools)
        let exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'pools')",
        )
        .fetch_one(&pool)
        .await
        .expect("check pools exists");

        assert!(exists.0, "expected 'pools' table to exist after migrations");

        // Revert migrations (undo all applied migrations).
        migrator.undo(&pool, 0).await.expect("undo migrations");

        // Confirm the table no longer exists
        let exists_after: (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'pools')",
        )
        .fetch_one(&pool)
        .await
        .expect("check pools exists after revert");

        assert!(
            !exists_after.0,
            "expected 'pools' table to be removed after revert"
        );
    }
}
