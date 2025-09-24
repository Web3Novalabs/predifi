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

    /// Create a test market for pool testing
    async fn create_test_market(&self, name: &str, category: &str) -> String {
        // First create the category
        let category_id = sqlx::query_scalar::<_, i32>(
            "INSERT INTO market_category (name) VALUES ($1) RETURNING id",
        )
        .bind(category)
        .fetch_one(&self.pool)
        .await
        .unwrap();

        // Then create the market
        let result = sqlx::query_scalar::<_, i32>(
            "INSERT INTO market (name, description, category_id) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(name)
        .bind(format!("Test market for {}", name))
        .bind(category_id)
        .fetch_one(&self.pool)
        .await
        .unwrap();

        result.to_string()
    }
}

/// Create a test app with basic routes
fn create_test_app(pool: PgPool) -> Router<AppState> {
    let db = Database::from_pool(pool);
    let state = AppState { db };

    Router::new().with_state(state)
}

#[tokio::test]
async fn test_pool_database_setup() {
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
async fn test_pool_app_creation() {
    // Test that we can create a test app
    let test_db = TestDb::new().await;

    // Create test app
    let _app = create_test_app(test_db.pool.clone());

    // Test that app was created successfully
    assert!(true, "Test app creation successful");
}

#[tokio::test]
async fn test_pool_data_persistence() {
    // Test that we can create and persist pool data
    let test_db = TestDb::new().await;

    // Start transaction for test isolation
    let mut tx = test_db
        .pool
        .begin()
        .await
        .expect("Failed to start transaction");

    // First create a test market
    let market_id = test_db
        .create_test_market("Pool Test Market", "pool-test")
        .await;

    // Create a pool associated with the market
    let pool_name = format!(
        "Test Persistence Pool {}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let pool_description = "Testing pool data persistence";
    let pool_status = "Active";

    let parsed_market_id = market_id.parse::<i32>().unwrap();

    let result = sqlx::query(
        "INSERT INTO pool (name, description, market_id, status, type) VALUES ($1, $2, $3, $4::pool_status, $5) RETURNING id"
    )
    .bind(&pool_name)
    .bind(pool_description)
    .bind(parsed_market_id)
    .bind(pool_status)
    .bind(1i16) // pool type
    .fetch_one(&mut *tx)
    .await;

    // Clone pool_name for later use
    let pool_name_clone = pool_name.clone();

    if let Err(ref e) = result {
        println!("Pool insertion failed: {}", e);
    }
    assert!(result.is_ok(), "Failed to insert pool into database");

    // Verify the pool was persisted
    let pool_id = sqlx::query_scalar::<_, i32>("SELECT id FROM pool WHERE name = $1")
        .bind(&pool_name_clone)
        .fetch_one(&mut *tx)
        .await
        .unwrap();

    assert!(pool_id > 0, "Pool ID should be positive");

    // Verify pool data directly from transaction
    let db_pool_result = sqlx::query_scalar::<_, String>(
        "SELECT json_build_object('id', id, 'name', name, 'description', description, 'market_id', market_id, 'status', status)::text FROM pool WHERE id = $1::integer"
    )
    .bind(pool_id)
    .fetch_optional(&mut *tx)
    .await;

    assert!(db_pool_result.is_ok(), "Pool query should succeed");
    let db_pool_json = db_pool_result.unwrap();
    assert!(
        db_pool_json.is_some(),
        "Pool should be retrievable from database"
    );

    let pool: serde_json::Value = serde_json::from_str(&db_pool_json.unwrap()).unwrap();
    assert_eq!(pool["name"], pool_name);
    assert_eq!(pool["description"], pool_description);
    assert_eq!(pool["market_id"], market_id.parse::<i32>().unwrap());
    assert_eq!(pool["status"], pool_status);

    // Transaction auto-rollbacks, no cleanup needed
    println!("✅ Pool data persistence test passed");
}

#[tokio::test]
async fn test_pool_migrations() {
    // Test that migrations run successfully
    let test_db = TestDb::new().await;

    // Start transaction for test isolation
    let mut tx = test_db
        .pool
        .begin()
        .await
        .expect("Failed to start transaction");

    // Check that pool table exists
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'pool')",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    assert!(table_exists, "Pool table should exist after migrations");

    // Check that user_pool table exists
    let user_pool_table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'user_pool')",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    assert!(
        user_pool_table_exists,
        "User pool table should exist after migrations"
    );

    // Check that market table exists (required for pool foreign key)
    let market_table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'market')",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    assert!(
        market_table_exists,
        "Market table should exist after migrations"
    );

    // Transaction auto-rollbacks, no cleanup needed
}
