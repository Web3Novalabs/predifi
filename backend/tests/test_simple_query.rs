use sqlx::PgPool;
use std::env;

#[tokio::test]
async fn test_simple_query() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    // Test basic connection
    let row: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("Basic query failed");
    assert_eq!(row.0, 1);

    // Test current database
    let db_info: (String, String) = sqlx::query_as("SELECT current_database(), current_user")
        .fetch_one(&pool)
        .await
        .expect("Database info query failed");

    println!("Connected to database: {}, user: {}", db_info.0, db_info.1);

    // Test if we can see the market table
    let table_exists: (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'market')",
    )
    .fetch_one(&pool)
    .await
    .expect("Table existence check failed");

    println!("Market table exists: {}", table_exists.0);

    // List all tables
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name"
    )
    .fetch_all(&pool)
    .await
    .expect("Table listing failed");

    println!("Available tables:");
    for (table_name,) in &tables {
        println!("  - {table_name}");
    }
}
