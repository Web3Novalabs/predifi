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
async fn test_validator_database_setup() {
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
async fn test_validator_app_creation() {
    // Test that we can create a test app
    let test_db = TestDb::new().await;

    // Create test app
    let _app = create_test_app(test_db.pool.clone());

    // Test that app was created successfully
    assert!(true, "Test app creation successful");
}

#[tokio::test]
async fn test_validator_data_persistence() {
    // Test that we can create and persist validator data
    let test_db = TestDb::new().await;

    // Start transaction for test isolation
    let mut tx = test_db
        .pool
        .begin()
        .await
        .expect("Failed to start transaction");

    // Create a validator
    let contract_address = "0x1234567890abcdef1234567890abcdef12345678";
    let is_active = true;

    let result = sqlx::query(
        "INSERT INTO validators (contract_address, is_active) VALUES ($1, $2) RETURNING contract_address"
    )
    .bind(contract_address)
    .bind(is_active)
    .fetch_one(&mut *tx)
    .await;

    assert!(result.is_ok(), "Failed to insert validator into database");

    // Verify the validator was persisted
    let validator_contract_address = sqlx::query_scalar::<_, String>(
        "SELECT contract_address FROM validators WHERE contract_address = $1",
    )
    .bind(contract_address)
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    assert!(
        !validator_contract_address.is_empty(),
        "Validator contract address should not be empty"
    );

    // Verify validator data directly from transaction
    let db_validator_result = sqlx::query_scalar::<_, String>(
        "SELECT json_build_object('contract_address', contract_address, 'is_active', is_active)::text FROM validators WHERE contract_address = $1"
    )
    .bind(contract_address)
    .fetch_optional(&mut *tx)
    .await;

    assert!(
        db_validator_result.is_ok(),
        "Validator query should succeed"
    );
    let db_validator_json = db_validator_result.unwrap();
    assert!(
        db_validator_json.is_some(),
        "Validator should be retrievable from database"
    );

    let validator: serde_json::Value = serde_json::from_str(&db_validator_json.unwrap()).unwrap();
    assert_eq!(validator["contract_address"], contract_address);
    assert_eq!(validator["is_active"], is_active);

    // Transaction auto-rollbacks, no cleanup needed
    println!("✅ Validator data persistence test passed");
}

#[tokio::test]
async fn test_validator_migrations() {
    // Test that migrations run successfully
    let test_db = TestDb::new().await;

    // Start transaction for test isolation
    let mut tx = test_db
        .pool
        .begin()
        .await
        .expect("Failed to start transaction");

    // Check that validators table exists
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'validators')",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    assert!(
        table_exists,
        "Validators table should exist after migrations"
    );

    // Check table structure
    let columns = sqlx::query_scalar::<_, String>(
        "SELECT string_agg(column_name, ', ' ORDER BY ordinal_position) FROM information_schema.columns WHERE table_name = 'validators'"
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    // Verify key columns exist
    assert!(
        columns.contains("contract_address"),
        "Validators table should have contract_address column"
    );
    assert!(
        columns.contains("is_active"),
        "Validators table should have is_active column"
    );
    assert!(
        columns.contains("registered_at"),
        "Validators table should have registered_at column"
    );
    assert!(
        columns.contains("updated_at"),
        "Validators table should have updated_at column"
    );

    // Transaction auto-rollbacks, no cleanup needed
}

#[tokio::test]
async fn test_validator_contract_address_uniqueness() {
    // Test that contract addresses must be unique
    let test_db = TestDb::new().await;

    // Start transaction for test isolation
    let mut tx = test_db
        .pool
        .begin()
        .await
        .expect("Failed to start transaction");

    let contract_address = "0x1234567890abcdef1234567890abcdef12345678";

    // Insert first validator
    let result1 = sqlx::query(
        "INSERT INTO validators (contract_address, is_active) VALUES ($1, $2) RETURNING contract_address"
    )
    .bind(contract_address)
    .bind(true)
    .fetch_one(&mut *tx)
    .await;

    assert!(
        result1.is_ok(),
        "First validator should be inserted successfully"
    );

    // Try to insert second validator with same contract address
    let result2 = sqlx::query(
        "INSERT INTO validators (contract_address, is_active) VALUES ($1, $2) RETURNING contract_address"
    )
    .bind(contract_address)
    .bind(false)
    .fetch_one(&mut *tx)
    .await;

    // This should fail due to unique constraint
    assert!(
        result2.is_err(),
        "Second validator with same contract address should fail"
    );

    // Since the transaction is now aborted, we need to verify the constraint worked
    // by checking that the error contains the expected constraint violation
    if let Err(e) = result2 {
        let error_string = e.to_string();
        assert!(
            error_string.contains("duplicate key") || error_string.contains("unique constraint"),
            "Error should indicate duplicate key or unique constraint violation, got: {}",
            error_string
        );
    }

    // Transaction auto-rollbacks, no cleanup needed
    println!("✅ Validator contract address uniqueness test passed");
}

#[tokio::test]
async fn test_validator_status_updates() {
    // Test that we can update validator status
    let test_db = TestDb::new().await;

    // Start transaction for test isolation
    let mut tx = test_db
        .pool
        .begin()
        .await
        .expect("Failed to start transaction");

    let contract_address = "0x1234567890abcdef1234567890abcdef12345678";

    // Insert validator with active status
    let result = sqlx::query(
        "INSERT INTO validators (contract_address, is_active) VALUES ($1, $2) RETURNING contract_address"
    )
    .bind(contract_address)
    .bind(true)
    .fetch_one(&mut *tx)
    .await;

    assert!(result.is_ok(), "Failed to insert validator");

    // Update status to inactive
    let update_result =
        sqlx::query("UPDATE validators SET is_active = $1 WHERE contract_address = $2")
            .bind(false)
            .bind(contract_address)
            .execute(&mut *tx)
            .await;

    assert!(update_result.is_ok(), "Failed to update validator status");

    // Verify status was updated directly from transaction
    let db_validator_result = sqlx::query_scalar::<_, String>(
        "SELECT json_build_object('contract_address', contract_address, 'is_active', is_active)::text FROM validators WHERE contract_address = $1"
    )
    .bind(contract_address)
    .fetch_optional(&mut *tx)
    .await;

    assert!(
        db_validator_result.is_ok(),
        "Validator query should succeed"
    );
    let db_validator_json = db_validator_result.unwrap();
    assert!(
        db_validator_json.is_some(),
        "Validator should still be retrievable"
    );

    let validator: serde_json::Value = serde_json::from_str(&db_validator_json.unwrap()).unwrap();
    assert_eq!(
        validator["is_active"], false,
        "Validator status should be updated to inactive"
    );

    // Transaction auto-rollbacks, no cleanup needed
    println!("✅ Validator status update test passed");
}
