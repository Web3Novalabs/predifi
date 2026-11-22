//! Integration tests for the migration safety preflight.
//!
//! These tests use a throwaway Postgres container to verify pending migration SQL,
//! preview the schema changes, and confirm transaction rollback leaves no changes.

#[cfg(test)]
mod tests {
    use predifi_backend::migration_checks::{run_rollback_test, CheckOptions, MigrationChecker};
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
    #[ignore = "Requires Docker container for Postgres"]
    async fn migration_preflight_previews_and_rolls_back() {
        let (pool, _container) = start_postgres().await;

        let report = MigrationChecker::new(pool.clone())
            .run(&CheckOptions::default())
            .await
            .expect("run migration safety check");

        let dry_run = report.dry_run.expect("dry-run result");
        assert!(dry_run.success, "{:?}", dry_run.error);
        assert!(dry_run.rollback_succeeded);
        assert!(report.pending_migrations > 0);
        assert!(report
            .schema_diff
            .iter()
            .any(|entry| matches!(entry, predifi_backend::migration_checks::SchemaDiffEntry::TableAdded(name) if name == "pools")));

        // Neither application objects nor SQLx bookkeeping should persist.
        let exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'pools')",
        )
        .fetch_one(&pool)
        .await
        .expect("check pools exists");

        assert!(!exists.0, "dry-run must not leave the pools table behind");
        let migration_table_exists: (bool,) =
            sqlx::query_as("SELECT to_regclass('public._sqlx_migrations') IS NOT NULL")
                .fetch_one(&pool)
                .await
                .expect("check SQLx migration history");
        assert!(!migration_table_exists.0);

        assert!(run_rollback_test(
            &pool,
            "CREATE TABLE rollback_probe (id BIGINT PRIMARY KEY); \
             CREATE INDEX rollback_probe_id_idx ON rollback_probe (id);"
        )
        .await
        .expect("rollback migration SQL"));
    }
}
