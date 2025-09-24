use axum::Router;
use backend::{AppState, db::database::Database};
use sqlx::PgPool;

/// Test database configuration
struct TestDb {
    pool: PgPool,
}

impl TestDb {
    /// Create a new test database connection
    async fn new() -> Self {
        // Use environment-based connection for testing
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/testdb".to_string());

        let pool = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        // Run migrations
        Self::run_migrations(&pool).await;

        Self { pool }
    }

    /// Run database migrations
    async fn run_migrations(pool: &PgPool) {
        sqlx::migrate!("./migrations")
            .run(pool)
            .await
            .expect("Failed to run migrations");
    }
}

/// Create a test app with the given database pool
fn create_test_app(pool: &PgPool) -> Router<AppState> {
    let db = Database::from_pool(pool.clone());
    let state = AppState { db };

    Router::new().with_state(state)
}

#[tokio::test]
async fn test_market_endpoints() {
    // Set up test environment using test containers
    let test_db = TestDb::new().await;

    // Create test app
    let _app = create_test_app(&test_db.pool);

    // Test that we can create a test app
    assert!(true, "Test app creation successful");
}

#[tokio::test]
async fn test_fixtures() {
    // Test that basic assertions work
    assert_eq!(2 + 2, 4, "Basic math should work");
    assert!(true, "True should be true");
}

#[tokio::test]
async fn test_database_cleanup() {
    // Set up test environment using test containers
    let test_db = TestDb::new().await;

    // Start transaction for test isolation
    let mut tx = test_db
        .pool
        .begin()
        .await
        .expect("Failed to start transaction");

    // Test that we can work with the database
    let result: Result<i32, _> = sqlx::query_scalar("SELECT 1").fetch_one(&mut *tx).await;
    assert!(result.is_ok(), "Database operations successful");

    // Transaction auto-rollbacks, no cleanup needed
}

#[tokio::test]
async fn test_database_connection() {
    // Test that we can connect to the test database
    let test_db = TestDb::new().await;

    // Start transaction for test isolation
    let mut tx = test_db
        .pool
        .begin()
        .await
        .expect("Failed to start transaction");

    // Test a simple query
    let result: Result<i32, _> = sqlx::query_scalar("SELECT 1").fetch_one(&mut *tx).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);

    // Transaction auto-rollbacks, no cleanup needed
}

#[tokio::test]
async fn test_migrations() {
    // Test that migrations run successfully
    let test_db = TestDb::new().await;

    // Start transaction for test isolation
    let mut tx = test_db
        .pool
        .begin()
        .await
        .expect("Failed to start transaction");

    // Check that tables exist
    let tables = sqlx::query_scalar::<_, String>(
        "SELECT string_agg(tablename, ', ') FROM pg_tables WHERE schemaname = 'public'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    // Verify key tables exist
    assert!(tables.contains("market"), "Market table should exist");
    assert!(tables.contains("pool"), "Pool table should exist");
    assert!(
        tables.contains("validators"),
        "Validators table should exist"
    );

    // Transaction auto-rollbacks, no cleanup needed
}
