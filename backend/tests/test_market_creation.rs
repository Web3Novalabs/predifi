use bigdecimal::BigDecimal;
use sqlx::PgPool;
use std::env;
use std::str::FromStr;

// Test isolation helper
async fn setup_test_environment() -> PgPool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    // Clean up any existing data before each test
    cleanup_all_test_data(&pool).await;

    pool
}

async fn cleanup_all_test_data(pool: &PgPool) {
    println!("Setting up clean test environment...");

    // Clean up in reverse order of dependencies to handle foreign key constraints
    sqlx::query("DELETE FROM market_tags")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM user_pool")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM pool").execute(pool).await.ok();
    sqlx::query("DELETE FROM tags").execute(pool).await.ok();
    sqlx::query("DELETE FROM market").execute(pool).await.ok();
    sqlx::query("DELETE FROM market_category")
        .execute(pool)
        .await
        .ok();

    println!("Clean test environment ready");
}

async fn create_test_market(
    pool: &PgPool,
    market_data: &serde_json::Value,
) -> Result<backend::models::market::MarketWithTags, sqlx::Error> {
    use backend::controllers::market_controller;
    use backend::models::market::NewMarket;

    let new_market = NewMarket {
        name: market_data["name"].as_str().unwrap().to_string(),
        description: market_data["description"].as_str().map(|s| s.to_string()),
        category_id: market_data["category_id"].as_i64().map(|i| i as i32),
        image_url: market_data["image_url"].as_str().map(|s| s.to_string()),
        event_source_url: market_data["event_source_url"]
            .as_str()
            .map(|s| s.to_string()),
        start_time: market_data["start_time"].as_i64(),
        lock_time: market_data["lock_time"].as_i64(),
        end_time: market_data["end_time"].as_i64(),
        option1: market_data["option1"].as_str().map(|s| s.to_string()),
        option2: market_data["option2"].as_str().map(|s| s.to_string()),
        min_bet_amount: market_data["min_bet_amount"]
            .as_str()
            .and_then(|s| BigDecimal::from_str(s).ok()),
        max_bet_amount: market_data["max_bet_amount"]
            .as_str()
            .and_then(|s| BigDecimal::from_str(s).ok()),
        creator_fee: market_data["creator_fee"].as_i64().map(|i| i as i16),
        is_private: market_data["is_private"].as_bool(),
        creator_address: market_data["creator_address"]
            .as_str()
            .map(|s| s.to_string()),
        created_timestamp: Some(chrono::Utc::now().timestamp()),
        tags: market_data["tags"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        }),
    };

    market_controller::create_market_with_tags(pool, &new_market).await
}

#[tokio::test]
#[ignore] // Temporarily ignore until we fix the parallel execution issue
async fn test_market_creation_with_tags() {
    let pool = setup_test_environment().await;

    // Test 1: Create market with new tags
    let market_data = serde_json::json!({
        "name": "Test Market Creation 1",
        "description": "A test market for integration testing",
        "category_id": null,
        "image_url": "https://example.com/image.jpg",
        "event_source_url": "https://example.com/event",
        "start_time": 1640995200, // 2022-01-01 00:00:00 UTC
        "lock_time": 1640998800,  // 2022-01-01 01:00:00 UTC
        "end_time": 1641002400,   // 2022-01-01 02:00:00 UTC
        "option1": "Yes",
        "option2": "No",
        "min_bet_amount": "0.1",
        "max_bet_amount": "100.0",
        "creator_fee": 5,
        "is_private": false,
        "creator_address": "0x1234567890abcdef",
        "tags": ["unique-test-sports", "unique-test-football", "unique-test-premier-league"]
    });

    // Simulate the market creation process
    let result = create_test_market(&pool, &market_data).await;
    assert!(result.is_ok(), "Market creation should succeed");

    let market_with_tags = result.unwrap();
    assert_eq!(market_with_tags.market.name, "Test Market Creation 1");
    assert_eq!(market_with_tags.tags.len(), 3);

    // Verify tags were created
    let tag_names: Vec<String> = market_with_tags
        .tags
        .iter()
        .map(|t| t.name.clone())
        .collect();
    assert!(tag_names.contains(&"unique-test-sports".to_string()));
    assert!(tag_names.contains(&"unique-test-football".to_string()));
    assert!(tag_names.contains(&"unique-test-premier-league".to_string()));

    // Test 2: Create another market with some existing tags
    let market_data_2 = serde_json::json!({
        "name": "Test Market Creation 2",
        "description": "Another test market",
        "category_id": null,
        "image_url": null,
        "event_source_url": null,
        "start_time": 1640995200,
        "lock_time": 1640998800,
        "end_time": 1641002400,
        "option1": "Team A",
        "option2": "Team B",
        "min_bet_amount": "0.5",
        "max_bet_amount": "50.0",
        "creator_fee": 3,
        "is_private": true,
        "creator_address": "0xabcdef1234567890",
        "tags": ["unique-test-sports", "unique-test-basketball", "unique-test-nba"] // "unique-test-sports" already exists
    });

    let result_2 = create_test_market(&pool, &market_data_2).await;
    assert!(result_2.is_ok(), "Second market creation should succeed");

    let market_with_tags_2 = result_2.unwrap();
    assert_eq!(market_with_tags_2.market.name, "Test Market Creation 2");
    assert_eq!(market_with_tags_2.tags.len(), 3);

    // Verify that the "unique-test-sports" tag was reused and new tags were created
    let tag_names_2: Vec<String> = market_with_tags_2
        .tags
        .iter()
        .map(|t| t.name.clone())
        .collect();
    assert!(tag_names_2.contains(&"unique-test-sports".to_string()));
    assert!(tag_names_2.contains(&"unique-test-basketball".to_string()));
    assert!(tag_names_2.contains(&"unique-test-nba".to_string()));

    // Test 3: Verify that tags table has the correct number of unique tags
    let unique_tags_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM tags WHERE name LIKE 'unique-test-%'")
            .fetch_one(&pool)
            .await
            .expect("Failed to count tags");

    // We should have 5 unique tags: unique-test-sports, unique-test-football, unique-test-premier-league, unique-test-basketball, unique-test-nba
    assert_eq!(
        unique_tags_count.0, 5,
        "Expected 5 unique test tags, found {}",
        unique_tags_count.0
    );

    // Test 4: Verify market_tags associations
    let market_tags_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM market_tags mt 
         JOIN tags t ON mt.tag_id = t.id 
         WHERE t.name LIKE 'unique-test-%'",
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to count market_tags");

    // We should have 6 associations: 3 for first market + 3 for second market
    assert_eq!(
        market_tags_count.0, 6,
        "Expected 6 unique test market-tag associations, found {}",
        market_tags_count.0
    );

    // Test 5: Test concurrent market creation
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let pool_clone = pool.clone();
            let market_data = serde_json::json!({
                "name": format!("Concurrent Creation Market {}", i),
                "description": format!("Concurrent test market {}", i),
                "category_id": null,
                "image_url": null,
                "event_source_url": null,
                "start_time": 1640995200,
                "lock_time": 1640998800,
                "end_time": 1641002400,
                "option1": "Option A",
                "option2": "Option B",
                "min_bet_amount": "1.0",
                "max_bet_amount": "10.0",
                "creator_fee": 2,
                "is_private": false,
                "creator_address": format!("0x{}", i),
                "tags": ["unique-test-concurrent", "unique-test-test", format!("unique-test-tag-{}", i)]
            });

            tokio::spawn(async move { create_test_market(&pool_clone, &market_data).await })
        })
        .collect();

    let results = futures::future::join_all(handles).await;
    for result in results {
        assert!(result.is_ok(), "Concurrent market creation should succeed");
    }

    // Verify all concurrent markets were created
    let total_markets: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM market WHERE name LIKE 'Concurrent Creation Market%'")
            .fetch_one(&pool)
            .await
            .expect("Failed to count markets");

    // 5 concurrent markets
    assert_eq!(total_markets.0, 5);

    // Clean up after test
    cleanup_all_test_data(&pool).await;
}

#[tokio::test]
async fn test_market_retrieval_with_tags() {
    let pool = setup_test_environment().await;

    // Create a test market with tags
    let market_data = serde_json::json!({
        "name": "Retrieval Test Market",
        "description": "Market for testing retrieval",
        "category_id": null,
        "image_url": "https://example.com/image.jpg",
        "event_source_url": "https://example.com/event",
        "start_time": 1640995200,
        "lock_time": 1640998800,
        "end_time": 1641002400,
        "option1": "Yes",
        "option2": "No",
        "min_bet_amount": "0.1",
        "max_bet_amount": "100.0",
        "creator_fee": 5,
        "is_private": false,
        "creator_address": "0x1234567890abcdef",
        "tags": ["retrieval-test", "retrieval-integration", "retrieval-verification"]
    });

    let created_market = create_test_market(&pool, &market_data).await.unwrap();
    let market_id = created_market.market.id;

    // Test retrieval
    let retrieved_market =
        backend::controllers::market_controller::get_market_with_tags(&pool, market_id).await;
    assert!(retrieved_market.is_ok(), "Market retrieval should succeed");

    let market_with_tags = retrieved_market.unwrap();
    assert_eq!(market_with_tags.market.id, market_id);
    assert_eq!(market_with_tags.market.name, "Retrieval Test Market");
    assert_eq!(market_with_tags.tags.len(), 3);

    // Verify all fields are correctly stored and retrieved
    assert_eq!(
        market_with_tags.market.description,
        Some("Market for testing retrieval".to_string())
    );
    assert_eq!(
        market_with_tags.market.image_url,
        Some("https://example.com/image.jpg".to_string())
    );
    assert_eq!(
        market_with_tags.market.event_source_url,
        Some("https://example.com/event".to_string())
    );
    assert_eq!(market_with_tags.market.start_time, Some(1640995200));
    assert_eq!(market_with_tags.market.lock_time, Some(1640998800));
    assert_eq!(market_with_tags.market.end_time, Some(1641002400));
    assert_eq!(market_with_tags.market.option1, Some("Yes".to_string()));
    assert_eq!(market_with_tags.market.option2, Some("No".to_string()));
    assert_eq!(market_with_tags.market.creator_fee, Some(5));
    assert_eq!(market_with_tags.market.is_private, Some(false));
    assert_eq!(
        market_with_tags.market.creator_address,
        Some("0x1234567890abcdef".to_string())
    );

    // Clean up after test
    cleanup_all_test_data(&pool).await;
}
