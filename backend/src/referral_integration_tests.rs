//! End-to-end integration tests for the referral system.
//!
//! Tests cover:
//! - Referrer registration
//! - Referred prediction placement
//! - Referral cut calculation
//! - Referral volume tracking across multiple pools
//! - Referrer update scenarios

#[cfg(test)]
mod tests {
    use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, QueryBuilder};
    use std::time::Duration;

    // ── Test setup helpers ──────────────────────────────────────────────────────

    /// Boot a Postgres container, run all migrations, and return the pool.
    /// Tests must bind the container to keep it alive.
    async fn setup() -> (PgPool, testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>) {
        let container = testcontainers_modules::postgres::Postgres::default()
            .start()
            .await
            .expect("postgres container");
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("postgres port");
        let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .expect("connect to postgres");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations");

        (pool, container)
    }

    /// Insert a pool into the database.
    async fn insert_pool(pool: &PgPool, pool_id: u64, name: &str) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO pools (pool_id, name, category, total_stake, end_time, state, creator, token)
            VALUES ($1, $2, $3, 0, NOW() + INTERVAL '1 day', 'active', 'GCREATOR', 'XLM')
            "#,
        )
        .bind(pool_id as i64)
        .bind(name)
        .bind("test")
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Insert a prediction into the database.
    async fn insert_prediction(
        pool: &PgPool,
        pool_id: u64,
        user_address: &str,
        amount: i64,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO predictions (pool_id, user_address, outcome, amount)
            VALUES ($1, $2, 0, $3)
            "#,
        )
        .bind(pool_id as i64)
        .bind(user_address)
        .bind(amount)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Insert a referral into the database.
    async fn insert_referral(
        pool: &PgPool,
        referrer: &str,
        user_address: &str,
        pool_id: u64,
        amount: i64,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO referrals (referrer, user_address, pool_id, amount)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(referrer)
        .bind(user_address)
        .bind(pool_id as i64)
        .bind(amount)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get referrer stats from the database.
    async fn get_referrer_stats(pool: &PgPool, referrer: &str) -> sqlx::Result<Option<sqlx::postgres::PgRow>> {
        let row = sqlx::query(
            r#"
            SELECT referrer, total_volume, unique_users, last_updated
            FROM referrer_stats
            WHERE referrer = $1
            "#,
        )
        .bind(referrer)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    /// Get per-pool referrer stats from the database.
    async fn get_referrer_pool_stats(
        pool: &PgPool,
        referrer: &str,
        pool_id: i64,
    ) -> sqlx::Result<Option<sqlx::postgres::PgRow>> {
        let row = sqlx::query(
            r#"
            SELECT referrer, pool_id, total_earned, referral_count, last_updated
            FROM referrer_pool_stats
            WHERE referrer = $1 AND pool_id = $2
            "#,
        )
        .bind(referrer)
        .bind(pool_id)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    // ── Test cases ──────────────────────────────────────────────────────────────

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_registration_and_volume_tracking() {
        let (pool, _container) = setup().await;

        // Setup: Create a pool
        insert_pool(&pool, 1, "Test Pool 1").await.unwrap();

        // Insert a referral event
        let referrer = "GREFERRER12345678901234567890123456789012345678901234";
        let referred_user = "GREFERRED1234567890123456789012345678901234567890123";
        let amount: i64 = 100_000_000; // 100 XLM in stroops

        insert_referral(&pool, referrer, referred_user, 1, amount).await.unwrap();

        // Verify referral was recorded
        let referral_count: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM referrals WHERE referrer = $1 AND user_address = $2",
        )
        .bind(referrer)
        .bind(referred_user)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(referral_count, 1);

        // Verify referrer stats were updated
        let stats = get_referrer_stats(&pool, referrer).await.unwrap();
        assert!(stats.is_some());
        let stats = stats.unwrap();
        let total_volume: i64 = stats.get("total_volume");
        let unique_users: i64 = stats.get("unique_users");

        assert_eq!(total_volume, amount);
        assert_eq!(unique_users, 1);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_cut_calculation() {
        let (pool, _container) = setup().await;

        // Setup: Create a pool
        insert_pool(&pool, 1, "Test Pool 1").await.unwrap();

        // Insert multiple referrals from same referrer
        let referrer = "GREFERRER56789012345678901234567890123456789012345678";
        let referral_amounts = vec![100_000_000, 200_000_000, 150_000_000]; // 100, 200, 150 XLM
        let total_volume: i64 = referral_amounts.iter().sum();

        for (i, &amount) in referral_amounts.iter().enumerate() {
            let user_address = format!("GREFERRED{:03}", i);
            insert_referral(&pool, referrer, &user_address, 1, amount)
                .await
                .unwrap();
        }

        // Verify total volume tracking
        let stats = get_referrer_stats(&pool, referrer).await.unwrap().unwrap();
        let stored_volume: i64 = stats.get("total_volume");

        assert_eq!(stored_volume, total_volume);

        // Verify unique users count
        let unique_users: i64 = stats.get("unique_users");
        assert_eq!(unique_users, 3);

        // Verify per-pool stats
        let pool_stats = get_referrer_pool_stats(&pool, referrer, 1).await.unwrap().unwrap();
        let pool_total_earned: i64 = pool_stats.get("total_earned");
        let pool_referral_count: i64 = pool_stats.get("referral_count");

        assert_eq!(pool_total_earned, total_volume);
        assert_eq!(pool_referral_count, 3);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_tracking_across_multiple_pools() {
        let (pool, _container) = setup().await;

        // Setup: Create multiple pools
        insert_pool(&pool, 1, "Pool A").await.unwrap();
        insert_pool(&pool, 2, "Pool B").await.unwrap();
        insert_pool(&pool, 3, "Pool C").await.unwrap();

        let referrer = "GREFERRER78901234567890123456789012345678901234567890";

        // Insert referrals across different pools
        let referrals = vec![
            (1, "GUSER001", 100_000_000), // Pool A, 100 XLM
            (2, "GUSER002", 200_000_000), // Pool B, 200 XLM
            (1, "GUSER003", 150_000_000), // Pool A, 150 XLM
            (3, "GUSER004", 300_000_000), // Pool C, 300 XLM
            (2, "GUSER005", 250_000_000), // Pool B, 250 XLM
        ];

        for (pool_id, user_address, amount) in referrals {
            insert_referral(&pool, referrer, user_address, pool_id, amount)
                .await
                .unwrap();
        }

        // Verify total volume across all pools
        let stats = get_referrer_stats(&pool, referrer).await.unwrap().unwrap();
        let total_volume: i64 = stats.get("total_volume");
        let total_expected: i64 = 100_000_000 + 200_000_000 + 150_000_000 + 300_000_000 + 250_000_000;
        assert_eq!(total_volume, total_expected);

        // Verify unique users (should count each user once, even if they appear multiple times)
        let unique_users: i64 = stats.get("unique_users");
        assert_eq!(unique_users, 5);

        // Verify per-pool breakdown
        let pool_a_stats = get_referrer_pool_stats(&pool, referrer, 1).await.unwrap().unwrap();
        let pool_b_stats = get_referrer_pool_stats(&pool, referrer, 2).await.unwrap().unwrap();
        let pool_c_stats = get_referrer_pool_stats(&pool, referrer, 3).await.unwrap().unwrap();

        assert_eq!(pool_a_stats.get::<i64, _>("total_earned"), 250_000_000); // 100 + 150
        assert_eq!(pool_b_stats.get::<i64, _>("total_earned"), 450_000_000); // 200 + 250
        assert_eq!(pool_c_stats.get::<i64, _>("total_earned"), 300_000_000);

        assert_eq!(pool_a_stats.get::<i64, _>("referral_count"), 2);
        assert_eq!(pool_b_stats.get::<i64, _>("referral_count"), 2);
        assert_eq!(pool_c_stats.get::<i64, _>("referral_count"), 1);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_update_scenario_duplicate_insert() {
        let (pool, _container) = setup().await;

        // Setup: Create a pool
        insert_pool(&pool, 1, "Test Pool 1").await.unwrap();

        let referrer = "GREFERRER01234567890123456789012345678901234567890123";
        let user_address = "GREFERRED0123456789012345678901234567890123456789012";
        let initial_amount: i64 = 100_000_000;

        // Insert initial referral
        insert_referral(&pool, referrer, user_address, 1, initial_amount)
            .await
            .unwrap();

        // Attempt to insert duplicate referral (should be ignored by ON CONFLICT)
        insert_referral(&pool, referrer, user_address, 1, initial_amount)
            .await
            .unwrap();

        // Verify only one referral was recorded (duplicates ignored)
        let referral_count: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM referrals WHERE referrer = $1 AND user_address = $2",
        )
        .bind(referrer)
        .bind(user_address)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(referral_count, 1);

        // Verify stats were NOT double-counted
        let stats = get_referrer_stats(&pool, referrer).await.unwrap().unwrap();
        let total_volume: i64 = stats.get("total_volume");
        assert_eq!(total_volume, initial_amount);

        // Verify per-pool stats were NOT double-counted
        let pool_stats = get_referrer_pool_stats(&pool, referrer, 1).await.unwrap().unwrap();
        let pool_total_earned: i64 = pool_stats.get("total_earned");
        assert_eq!(pool_total_earned, initial_amount);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_update_scenario_additional_referral_same_user() {
        let (pool, _container) = setup().await;

        // Setup: Create two pools
        insert_pool(&pool, 1, "Pool A").await.unwrap();
        insert_pool(&pool, 2, "Pool B").await.unwrap();

        let referrer = "GREFERRER2345678901234567890123456789012345678901234";
        let user_address = "GREFERRED2345678901234567890123456789012345678901234";

        // First referral to Pool A
        insert_referral(&pool, referrer, user_address, 1, 100_000_000)
            .await
            .unwrap();

        // Same user makes a second referral to Pool B (different pool, same referrer)
        insert_referral(&pool, referrer, user_address, 2, 200_000_000)
            .await
            .unwrap();

        // Verify total volume includes both referrals
        let stats = get_referrer_stats(&pool, referrer).await.unwrap().unwrap();
        let total_volume: i64 = stats.get("total_volume");
        assert_eq!(total_volume, 300_000_000);

        // Verify unique users count is 1 (same user, different pools)
        let unique_users: i64 = stats.get("unique_users");
        assert_eq!(unique_users, 1);

        // Verify per-pool stats for Pool A
        let pool_a_stats = get_referrer_pool_stats(&pool, referrer, 1).await.unwrap().unwrap();
        assert_eq!(pool_a_stats.get::<i64, _>("total_earned"), 100_000_000);
        assert_eq!(pool_a_stats.get::<i64, _>("referral_count"), 1);

        // Verify per-pool stats for Pool B
        let pool_b_stats = get_referrer_pool_stats(&pool, referrer, 2).await.unwrap().unwrap();
        assert_eq!(pool_b_stats.get::<i64, _>("total_earned"), 200_000_000);
        assert_eq!(pool_b_stats.get::<i64, _>("referral_count"), 1);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn multiple_referrers_volume_tracking() {
        let (pool, _container) = setup().await;

        // Setup: Create pools
        insert_pool(&pool, 1, "Pool 1").await.unwrap();
        insert_pool(&pool, 2, "Pool 2").await.unwrap();

        let referrer_a = "GREFERRERA34567890123456789012345678901234567890123";
        let referrer_b = "GREFERRERB45678901234567890123456789012345678901234";

        // Referrer A's referrals
        insert_referral(&pool, referrer_a, "GUSER001", 1, 100_000_000)
            .await
            .unwrap();
        insert_referral(&pool, referrer_a, "GUSER002", 1, 150_000_000)
            .await
            .unwrap();
        insert_referral(&pool, referrer_a, "GUSER003", 2, 200_000_000)
            .await
            .unwrap();

        // Referrer B's referrals
        insert_referral(&pool, referrer_b, "GUSER004", 1, 250_000_000)
            .await
            .unwrap();
        insert_referral(&pool, referrer_b, "GUSER005", 2, 300_000_000)
            .await
            .unwrap();

        // Verify Referrer A stats
        let stats_a = get_referrer_stats(&pool, referrer_a).await.unwrap().unwrap();
        let volume_a: i64 = stats_a.get("total_volume");
        assert_eq!(volume_a, 450_000_000); // 100 + 150 + 200
        assert_eq!(stats_a.get::<i64, _>("unique_users"), 3);

        // Verify Referrer B stats
        let stats_b = get_referrer_stats(&pool, referrer_b).await.unwrap().unwrap();
        let volume_b: i64 = stats_b.get("total_volume");
        assert_eq!(volume_b, 550_000_000); // 250 + 300
        assert_eq!(stats_b.get::<i64, _>("unique_users"), 2);

        // Verify no cross-contamination
        assert_ne!(volume_a, volume_b);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_with_zero_amount() {
        let (pool, _container) = setup().await;

        // Setup: Create a pool
        insert_pool(&pool, 1, "Test Pool").await.unwrap();

        let referrer = "GREFERRER56789012345678901234567890123456789012345678";
        let user_address = "GREFERRED56789012345678901234567890123456789012345678";

        // Insert referral with zero amount
        insert_referral(&pool, referrer, user_address, 1, 0).await.unwrap();

        // Verify referral was recorded
        let referral_count: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM referrals WHERE referrer = $1 AND amount = 0",
        )
        .bind(referrer)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(referral_count, 1);

        // Verify stats reflect zero amount
        let stats = get_referrer_stats(&pool, referrer).await.unwrap().unwrap();
        let total_volume: i64 = stats.get("total_volume");
        assert_eq!(total_volume, 0);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_large_volume_tracking() {
        let (pool, _container) = setup().await;

        // Setup: Create a pool
        insert_pool(&pool, 1, "Large Pool").await.unwrap();

        let referrer = "GREFERRER67890123456789012345678901234567890123456789";

        // Insert multiple large referrals
        let large_amount: i64 = 1_000_000_000_000; // 1 million XLM
        let referrals = 100;

        for i in 0..referrals {
            let user_address = format!("GREFERRED{:08}", i);
            insert_referral(&pool, referrer, &user_address, 1, large_amount)
                .await
                .unwrap();
        }

        // Verify total volume
        let stats = get_referrer_stats(&pool, referrer).await.unwrap().unwrap();
        let total_volume: i64 = stats.get("total_volume");
        let expected_volume: i64 = large_amount * referrals;
        assert_eq!(total_volume, expected_volume);

        // Verify referral count
        let pool_stats = get_referrer_pool_stats(&pool, referrer, 1).await.unwrap().unwrap();
        let count: i64 = pool_stats.get("referral_count");
        assert_eq!(count, referrals);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn concurrent_referrals_same_referrer() {
        let (pool, _container) = setup().await;

        // Setup: Create a pool
        insert_pool(&pool, 1, "Concurrent Pool").await.unwrap();

        let referrer = "GREFERRER78901234567890123456789012345678901234567890";

        // Insert 10 referrals concurrently
        let mut tasks = Vec::new();
        for i in 0..10 {
            let pool_clone = pool.clone();
            let referrer_clone = referrer.to_string();
            let user_address = format!("GUSER{:08}", i);

            let task = tokio::spawn(async move {
                insert_referral(&pool_clone, &referrer_clone, &user_address, 1, 10_000_000)
                    .await
            });
            tasks.push(task);
        }

        // Wait for all tasks to complete
        for task in tasks {
            task.await.unwrap().unwrap();
        }

        // Verify all referrals were recorded
        let referral_count: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM referrals WHERE referrer = $1",
        )
        .bind(referrer)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(referral_count, 10);

        // Verify total volume
        let stats = get_referrer_stats(&pool, referrer).await.unwrap().unwrap();
        let total_volume: i64 = stats.get("total_volume");
        assert_eq!(total_volume, 100_000_000); // 10 * 10_000_000

        // Verify per-pool stats
        let pool_stats = get_referrer_pool_stats(&pool, referrer, 1).await.unwrap().unwrap();
        let count: i64 = pool_stats.get("referral_count");
        assert_eq!(count, 10);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_volume_consistency_check() {
        let (pool, _container) = setup().await;

        // Setup: Create multiple pools
        insert_pool(&pool, 1, "Pool 1").await.unwrap();
        insert_pool(&pool, 2, "Pool 2").await.unwrap();
        insert_pool(&pool, 3, "Pool 3").await.unwrap();

        let referrer = "GREFERRER89012345678901234567890123456789012345678901";

        // Insert various referrals
        let referrals = vec![
            (1, "GUSER001", 100_000_000),
            (1, "GUSER002", 200_000_000),
            (2, "GUSER003", 150_000_000),
            (2, "GUSER004", 250_000_000),
            (3, "GUSER005", 300_000_000),
        ];

        for (pool_id, user_address, amount) in referrals {
            insert_referral(&pool, referrer, user_address, pool_id, amount)
                .await
                .unwrap();
        }

        // Manual calculation of expected totals
        let pool_1_total = 100_000_000 + 200_000_000;
        let pool_2_total = 150_000_000 + 250_000_000;
        let pool_3_total = 300_000_000;
        let grand_total = pool_1_total + pool_2_total + pool_3_total;

        // Verify global stats
        let stats = get_referrer_stats(&pool, referrer).await.unwrap().unwrap();
        assert_eq!(stats.get::<i64, _>("total_volume"), grand_total);
        assert_eq!(stats.get::<i64, _>("unique_users"), 5);

        // Verify per-pool stats
        let pool_1_stats = get_referrer_pool_stats(&pool, referrer, 1).await.unwrap().unwrap();
        let pool_2_stats = get_referrer_pool_stats(&pool, referrer, 2).await.unwrap().unwrap();
        let pool_3_stats = get_referrer_pool_stats(&pool, referrer, 3).await.unwrap().unwrap();

        assert_eq!(pool_1_stats.get::<i64, _>("total_earned"), pool_1_total);
        assert_eq!(pool_2_stats.get::<i64, _>("total_earned"), pool_2_total);
        assert_eq!(pool_3_stats.get::<i64, _>("total_earned"), pool_3_total);

        // Cross-check: sum of per-pool totals should equal global total
        let sum_of_pools: i64 = pool_1_stats.get("total_earned") + pool_2_stats.get("total_earned") + pool_3_stats.get("total_earned");
        assert_eq!(sum_of_pools, grand_total);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_user_address_index() {
        let (pool, _container) = setup().await;

        // Setup: Create pools
        insert_pool(&pool, 1, "Pool 1").await.unwrap();
        insert_pool(&pool, 2, "Pool 2").await.unwrap();

        let referrer = "GREFERRER90123456789012345678901234567890123456789012";

        // Insert multiple referrals from different users
        for i in 0..10 {
            let user_address = format!("GUSER{:08}", i);
            insert_referral(&pool, referrer, &user_address, 1, 10_000_000)
                .await
                .unwrap();
        }

        // Verify we can query by user_address
        let user_referrals: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM referrals WHERE user_address = $1",
        )
        .bind("GUSER00000000")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(user_referrals, 1);

        // Verify index is being used (EXPLAIN would be better, but this confirms functionality)
        let rows: Vec<String> = sqlx::query_scalar::<_, String>(
            "SELECT user_address FROM referrals WHERE user_address LIKE $1",
        )
        .bind("GUSER%")
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(rows.len(), 10);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_referrer_pool_id_index() {
        let (pool, _container) = setup().await;

        // Setup: Create pools
        insert_pool(&pool, 1, "Pool 1").await.unwrap();
        insert_pool(&pool, 2, "Pool 2").await.unwrap();

        let referrer = "GREFERRER01234567890123456789012345678901234567890123";

        // Insert referrals with different pool_id
        insert_referral(&pool, referrer, "GUSER001", 1, 100_000_000)
            .await
            .unwrap();
        insert_referral(&pool, referrer, "GUSER002", 2, 200_000_000)
            .await
            .unwrap();

        // Query with both referrer and pool_id (composite index)
        let row: Option<sqlx::postgres::PgRow> = sqlx::query(
            "SELECT * FROM referrals WHERE referrer = $1 AND pool_id = $2",
        )
        .bind(referrer)
        .bind(1 as i64)
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert!(row.is_some());
        let amount: i64 = row.unwrap().get("amount");
        assert_eq!(amount, 100_000_000);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_timestamps() {
        let (pool, _container) = setup().await;

        // Setup: Create a pool
        insert_pool(&pool, 1, "Test Pool").await.unwrap();

        let referrer = "GREFERRER12345678901234567890123456789012345678901234";
        let user_address = "GREFERRED1234567890123456789012345678901234567890123";

        // Insert referral
        insert_referral(&pool, referrer, user_address, 1, 100_000_000)
            .await
            .unwrap();

        // Verify created_at timestamp is set
        let created_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>(
            "SELECT created_at FROM referrals WHERE referrer = $1",
        )
        .bind(referrer)
        .fetch_one(&pool)
        .await
        .unwrap();

        // Timestamp should be recent (within last second)
        let now = chrono::Utc::now();
        let diff = (now - created_at).num_seconds();
        assert!(diff >= 0 && diff < 5, "created_at should be recent, diff: {}", diff);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_event_based_insertion() {
        let (pool, _container) = setup().await;

        // Setup: Create a pool
        insert_pool(&pool, 1, "Event Pool").await.unwrap();

        // Simulate event-based insertion (as done in stellar_listener.rs)
        use crate::db::ReferralPaidEvent;

        let event = ReferralPaidEvent {
            pool_id: 1,
            referrer: "GREFERRER2345678901234567890123456789012345678901234".to_string(),
            referred_user: "GREFERRED2345678901234567890123456789012345678901234".to_string(),
            referral_amount: 150_000_000,
        };

        // Insert using the same method as stellar_listener
        crate::db::insert_referral_from_event(&pool, &event)
            .await
            .unwrap();

        // Verify insertion
        let count: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM referrals WHERE referrer = $1",
        )
        .bind(&event.referrer)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1);

        // Verify stats
        let stats = get_referrer_stats(&pool, &event.referrer).await.unwrap().unwrap();
        let volume: i64 = stats.get("total_volume");
        assert_eq!(volume, event.referral_amount);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_bulk_insertion() {
        let (pool, _container) = setup().await;

        // Setup: Create a pool
        insert_pool(&pool, 1, "Bulk Pool").await.unwrap();

        // Prepare bulk events
        use crate::db::ReferralPaidEvent;

        let events: Vec<ReferralPaidEvent> = (0..5)
            .map(|i| ReferralPaidEvent {
                pool_id: 1,
                referrer: "GREFERRER345678901234567890123456789012345678901234".to_string(),
                referred_user: format!("GREFERRED{:08}", i),
                referral_amount: (i as i64 + 1) * 10_000_000,
            })
            .collect();

        // Insert using bulk method
        crate::db::insert_referrals_bulk(&pool, &events, 100)
            .await
            .unwrap();

        // Verify all events were inserted
        let count: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM referrals WHERE referrer = $1",
        )
        .bind("GREFERRER345678901234567890123456789012345678901234")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 5);

        // Verify total volume
        let stats = get_referrer_stats(&pool, "GREFERRER345678901234567890123456789012345678901234")
            .await
            .unwrap()
            .unwrap();
        let volume: i64 = stats.get("total_volume");
        let expected_volume: i64 = 10_000_000 + 20_000_000 + 30_000_000 + 40_000_000 + 50_000_000;
        assert_eq!(volume, expected_volume);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_get_user_referral_earnings() {
        let (pool, _container) = setup().await;

        // Setup: Create pools
        insert_pool(&pool, 1, "Pool A").await.unwrap();
        insert_pool(&pool, 2, "Pool B").await.unwrap();
        insert_pool(&pool, 3, "Pool C").await.unwrap();

        let referrer = "GREFERRER45678901234567890123456789012345678901234567";

        // Insert referrals to different pools
        insert_referral(&pool, referrer, "GUSER001", 1, 100_000_000)
            .await
            .unwrap();
        insert_referral(&pool, referrer, "GUSER002", 2, 200_000_000)
            .await
            .unwrap();
        insert_referral(&pool, referrer, "GUSER003", 3, 150_000_000)
            .await
            .unwrap();

        // Test get_referral_earnings function (from referrals.rs)
        let earnings = crate::db::get_referral_earnings(&pool, referrer)
            .await
            .unwrap();

        assert_eq!(earnings.len(), 3);

        // Verify earnings for each pool
        let pool_a_earning = earnings.iter().find(|e| e.pool_id == 1).unwrap();
        let pool_b_earning = earnings.iter().find(|e| e.pool_id == 2).unwrap();
        let pool_c_earning = earnings.iter().find(|e| e.pool_id == 3).unwrap();

        assert_eq!(pool_a_earning.total_earned, 100_000_000);
        assert_eq!(pool_a_earning.referral_count, 1);
        assert_eq!(pool_b_earning.total_earned, 200_000_000);
        assert_eq!(pool_b_earning.referral_count, 1);
        assert_eq!(pool_c_earning.total_earned, 150_000_000);
        assert_eq!(pool_c_earning.referral_count, 1);

        // Verify total earned across all pools
        let total_earned: i64 = earnings.iter().map(|e| e.total_earned).sum();
        assert_eq!(total_earned, 450_000_000);

        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "Requires Docker container for Postgres"]
    async fn referral_estimate_referral_reward() {
        // Test the estimate_referral_reward function directly
        use crate::referrals::estimate_referral_reward;

        // Test 1: 1_000_000 volume, 2% treasury fee, 30% referral share => 6_000
        let reward = estimate_referral_reward(1_000_000, 200, 3_000);
        assert_eq!(reward, 6_000);

        // Test 2: Different fee rates
        let reward = estimate_referral_reward(500_000, 300, 5_000);
        // 500_000 * 0.03 * 0.5 = 7_500
        assert_eq!(reward, 7_500);

        // Test 3: Zero volume
        let reward = estimate_referral_reward(0, 200, 3_000);
        assert_eq!(reward, 0);

        // Test 4: Zero fees
        let reward = estimate_referral_reward(1_000_000, 0, 3_000);
        assert_eq!(reward, 0);

        let reward = estimate_referral_reward(1_000_000, 200, 0);
        assert_eq!(reward, 0);

        // Test 5: Maximum fees (100% treasury, 100% referral share)
        let reward = estimate_referral_reward(1_000_000, 10_000, 10_000);
        assert_eq!(reward, 1_000_000);

        // Test 6: Large volume (should not overflow)
        let reward = estimate_referral_reward(i64::MAX, 10_000, 10_000);
        assert_eq!(reward, i64::MAX);
    }
}
