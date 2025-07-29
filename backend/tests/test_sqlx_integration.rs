use sqlx::{Executor, PgPool};
use std::env;

#[tokio::test]
async fn test_database_connection_and_migration() {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to DB");
    // Check connection
    let row: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("Ping failed");
    assert_eq!(row.0, 1);
    // Check if market_category table exists
    let exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_name = 'market_category'
        )",
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to check table existence");
    assert!(exists.0, "market_category table should exist");
    // Insert and fetch a market_category
    let inserted: (i32, String) =
        sqlx::query_as("INSERT INTO market_category (name) VALUES ($1) RETURNING id, name")
            .bind("TestCategory")
            .fetch_one(&pool)
            .await
            .expect("Insert failed");
    assert_eq!(inserted.1, "TestCategory");
    // Clean up
    sqlx::query("DELETE FROM market_category WHERE id = $1")
        .bind(inserted.0)
        .execute(&pool)
        .await
        .expect("Cleanup failed");
}
