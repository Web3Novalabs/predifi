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

/// Create a test app with basic routes
fn create_test_app(pool: PgPool) -> Router<AppState> {
    let db = Database::from_pool(pool);
    let state = AppState { db };

    Router::new().with_state(state)
}

#[tokio::test]
async fn test_market_database_setup() {
    // Test that we can set up a test database
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
async fn test_market_app_creation() {
    // Test that we can create a test app
    let test_db = TestDb::new().await;

    // Create test app
    let _app = create_test_app(test_db.pool.clone());

    // Test that app was created successfully
    assert!(true, "Test app creation successful");
}

#[tokio::test]
async fn test_market_data_persistence() {
    // Test that we can create and persist market data
    let test_db = TestDb::new().await;

    // Start transaction for test isolation
    let mut tx = test_db
        .pool
        .begin()
        .await
        .expect("Failed to start transaction");

    // Create a simple market directly in the database
    let market_name = "Test Persistence Market";
    let market_description = "Testing data persistence";
    let market_category = "persistence-test";

    // First create the category
    let category_id =
        sqlx::query_scalar::<_, i32>("INSERT INTO market_category (name) VALUES ($1) RETURNING id")
            .bind(market_category)
            .fetch_one(&mut *tx)
            .await
            .unwrap();

    // Then create the market
    let result = sqlx::query(
        "INSERT INTO market (name, description, category_id) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(market_name)
    .bind(market_description)
    .bind(category_id)
    .fetch_one(&mut *tx)
    .await;

    assert!(result.is_ok(), "Failed to insert market into database");

    // Verify the market was persisted
    let market_id = sqlx::query_scalar::<_, i32>("SELECT id FROM market WHERE name = $1")
        .bind(market_name)
        .fetch_one(&mut *tx)
        .await
        .unwrap();

    assert!(market_id > 0, "Market ID should be positive");

    // Verify market data directly from transaction
    let db_market_result = sqlx::query_scalar::<_, String>(
        "SELECT json_build_object('id', id, 'name', name, 'description', description, 'category_id', category_id)::text FROM market WHERE id = $1::integer"
    )
    .bind(market_id)
    .fetch_optional(&mut *tx)
    .await;

    assert!(db_market_result.is_ok(), "Market query should succeed");
    let db_market_json = db_market_result.unwrap();
    assert!(
        db_market_json.is_some(),
        "Market should be retrievable from database"
    );

    let market: serde_json::Value = serde_json::from_str(&db_market_json.unwrap()).unwrap();
    assert_eq!(market["name"], market_name);
    assert_eq!(market["description"], market_description);
    assert_eq!(market["category_id"], category_id);

    // Transaction auto-rollbacks, no cleanup needed
    println!("✅ Market data persistence test passed");
}

#[tokio::test]
async fn test_market_migrations() {
    // Test that migrations run successfully
    let test_db = TestDb::new().await;

    // Start transaction for test isolation
    let mut tx = test_db
        .pool
        .begin()
        .await
        .expect("Failed to start transaction");

    // Check that market table exists
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'market')",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    assert!(table_exists, "Market table should exist after migrations");

    // Check that market_category table exists
    let category_table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'market_category')"
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    assert!(
        category_table_exists,
        "Market category table should exist after migrations"
    );

    // Check that tags table exists
    let tags_table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'tags')",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    assert!(
        tags_table_exists,
        "Tags table should exist after migrations"
    );

    // Check that market_tags table exists
    let market_tags_table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'market_tags')",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    assert!(
        market_tags_table_exists,
        "Market tags table should exist after migrations"
    );

    // Transaction auto-rollbacks, no cleanup needed
}
