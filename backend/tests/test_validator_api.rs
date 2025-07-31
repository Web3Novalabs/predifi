use backend::models::validator::Validator;
use dotenvy;
use sqlx::PgPool;
use std::env;

#[tokio::test]
async fn test_validator_api() {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    // Clean up before test
    sqlx::query("DELETE FROM validators")
        .execute(&pool)
        .await
        .unwrap();

    // Test getting all validators (empty)
    let validators: Vec<Validator> = sqlx::query_as("SELECT * FROM validators")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(validators.is_empty());

    // Insert a valid validator
    let address = "0x1234567890abcdef1234567890abcdef12345678";
    sqlx::query("INSERT INTO validators (contract_address, is_active) VALUES ($1, $2)")
        .bind(address)
        .bind(true)
        .execute(&pool)
        .await
        .unwrap();

    // Test getting validator by valid contract address
    let validator: Validator =
        sqlx::query_as("SELECT * FROM validators WHERE contract_address = $1")
            .bind(address)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(validator.contract_address, address);

    // Test getting validator with invalid/non-existent address
    let result =
        sqlx::query_as::<_, Validator>("SELECT * FROM validators WHERE contract_address = $1")
            .bind("0x0000000000000000000000000000000000000000")
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert!(result.is_none());

    // Test getting all validators (populated)
    let validators: Vec<Validator> = sqlx::query_as("SELECT * FROM validators")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(validators.len(), 1);

    // Test validator status endpoint with valid address
    let status: bool =
        sqlx::query_scalar("SELECT is_active FROM validators WHERE contract_address = $1")
            .bind(address)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(status);

    // Test validator status with non-validator address
    let status_result = sqlx::query_scalar::<_, bool>(
        "SELECT is_active FROM validators WHERE contract_address = $1",
    )
    .bind("0x0000000000000000000000000000000000000000")
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(status_result.is_none());

    // Test contract address format validation (simulate controller logic)
    let bad_address = "invalid";
    let is_valid = bad_address.starts_with("0x") && bad_address.len() == 42;
    assert!(!is_valid);

    // Clean up after test
    sqlx::query("DELETE FROM validators WHERE contract_address = $1")
        .bind(address)
        .execute(&pool)
        .await
        .unwrap();
}
