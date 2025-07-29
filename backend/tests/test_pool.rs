use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use backend::controllers::pool_controller::*;
use backend::models::pool::{NewPool, Pool, PoolStatus};
use backend::routes::pool_route::pool_routes;
use bigdecimal::BigDecimal;
use dotenvy;
use sqlx::{PgPool, Row};
use sqlx::{Pool as SqlxPool, Postgres};
use std::env;
use tower::ServiceExt; // for .oneshot()

#[tokio::test]
async fn test_pool_controller_functions() {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
    let pool = SqlxPool::<Postgres>::connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    // Insert a market row
    let market_row = sqlx::query("INSERT INTO market (name) VALUES ($1) RETURNING id")
        .bind("Test Market")
        .fetch_one(&pool)
        .await
        .expect("Insert market failed");
    let market_id: i32 = market_row.get("id");

    // Test create_pool
    let new_pool = NewPool {
        market_id,
        name: "Test Pool".to_string(),
        r#type: 1,
        description: Some("desc".to_string()),
        image_url: None,
        event_source_url: None,
        start_time: None,
        lock_time: None,
        end_time: None,
        option1: Some("A".to_string()),
        option2: Some("B".to_string()),
        min_bet_amount: Some(BigDecimal::from(1)),
        max_bet_amount: Some(BigDecimal::from(100)),
        creator_fee: Some(5),
        is_private: Some(false),
        category_id: None,
    };
    let pool_obj = create_pool(&pool, &new_pool)
        .await
        .expect("create_pool failed");
    assert_eq!(pool_obj.name, "Test Pool");

    // Test get_pool
    let fetched = get_pool(&pool, pool_obj.id).await.expect("get_pool failed");
    assert_eq!(fetched.id, pool_obj.id);

    // Insert pools with different statuses
    let statuses = [
        PoolStatus::Active,
        PoolStatus::Locked,
        PoolStatus::Settled,
        PoolStatus::Closed,
    ];
    let mut pool_ids = vec![pool_obj.id];
    for status in statuses.iter() {
        let row = sqlx::query("INSERT INTO pool (market_id, name, type, status) VALUES ($1, $2, $3, $4::pool_status) RETURNING id")
            .bind(market_id)
            .bind(format!("Pool-{:?}", status))
            .bind(1)
            .bind(format!("{:?}", status))
            .fetch_one(&pool)
            .await
            .expect("Insert pool failed");
        pool_ids.push(row.get::<i32, _>("id"));
    }

    // Test get_pools_by_status
    let active_pools = get_pools_by_status(&pool, "Active", 10, 0)
        .await
        .expect("get_pools_by_status failed");
    assert!(active_pools.iter().any(|p| p.status == PoolStatus::Active));

    // Test get_active_pools
    let actives = get_active_pools(&pool, 10, 0)
        .await
        .expect("get_active_pools failed");
    assert!(actives.iter().all(|p| p.status == PoolStatus::Active));

    // Test get_locked_pools
    let locked = get_locked_pools(&pool, 10, 0)
        .await
        .expect("get_locked_pools failed");
    assert!(locked.iter().all(|p| p.status == PoolStatus::Locked));

    // Test get_settled_pools
    let settled = get_settled_pools(&pool, 10, 0)
        .await
        .expect("get_settled_pools failed");
    assert!(settled.iter().all(|p| p.status == PoolStatus::Settled));

    // Test get_closed_pools
    let closed = get_closed_pools(&pool, 10, 0)
        .await
        .expect("get_closed_pools failed");
    assert!(closed.iter().all(|p| p.status == PoolStatus::Closed));

    // Test create_user_pool
    let user_pool = create_user_pool(&pool, "user1", pool_obj.id, &BigDecimal::from(10))
        .await
        .expect("create_user_pool failed");
    assert_eq!(user_pool.user_id, "user1");

    // Test get_user_pool
    let fetched_user_pool = get_user_pool(&pool, user_pool.id)
        .await
        .expect("get_user_pool failed");
    assert_eq!(fetched_user_pool.id, user_pool.id);

    // Clean up: delete user_pool rows before pool rows to avoid FK violation
    // Remove all user_pool rows referencing any pool in pool_ids
    sqlx::query("DELETE FROM user_pool WHERE pool_id = ANY($1)")
        .bind(&pool_ids)
        .execute(&pool)
        .await
        .expect("Cleanup user_pool failed");
    for id in pool_ids {
        sqlx::query("DELETE FROM pool WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .expect("Cleanup pool failed");
    }
    sqlx::query("DELETE FROM market WHERE id = $1")
        .bind(market_id)
        .execute(&pool)
        .await
        .expect("Cleanup market failed");
}

#[tokio::test]
async fn test_pools_routes() {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    // Insert a market row
    let market_row = sqlx::query("INSERT INTO market (name) VALUES ($1) RETURNING id")
        .bind("RouteTest Market")
        .fetch_one(&pool)
        .await
        .expect("Insert market failed");
    let market_id: i32 = market_row.get("id");

    // Insert pools with different statuses
    let statuses = ["Active", "Locked", "Settled", "Closed"];
    let mut pool_ids = vec![];
    for status in statuses.iter() {
        let row = sqlx::query("INSERT INTO pool (market_id, name, type, status) VALUES ($1, $2, $3, $4::pool_status) RETURNING id")
            .bind(market_id)
            .bind(format!("Pool-{}", status))
            .bind(1)
            .bind(*status)
            .fetch_one(&pool)
            .await
            .expect("Insert pool failed");
        pool_ids.push(row.get::<i32, _>("id"));
    }

    // Build app state and router
    let state = backend::db::database::AppState {
        db: backend::db::database::Database { pool: pool.clone() },
    };
    let app = pool_routes().with_state(state);

    // Helper to test each route
    async fn test_route(app: &Router, route: &str, expected_status: &str) {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(route).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = std::str::from_utf8(&body).unwrap();
        assert!(
            body_str.contains(expected_status),
            "Response for {} should contain status {}",
            route,
            expected_status
        );
    }

    test_route(&app, "/pools/active", "Active").await;
    test_route(&app, "/pools/locked", "Locked").await;
    test_route(&app, "/pools/settled", "Settled").await;
    test_route(&app, "/pools/closed", "Closed").await;

    // Clean up
    for id in pool_ids {
        sqlx::query("DELETE FROM pool WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .expect("Cleanup pool failed");
    }
    sqlx::query("DELETE FROM market WHERE id = $1")
        .bind(market_id)
        .execute(&pool)
        .await
        .expect("Cleanup market failed");
}

#[tokio::test]
async fn test_pool_status_enum_mapping() {
    dotenvy::dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to DB");

    // Insert a market row to satisfy foreign key constraint
    let market_row = sqlx::query("INSERT INTO market (name) VALUES ($1) RETURNING id")
        .bind("Test Market")
        .fetch_one(&pool)
        .await
        .expect("Insert market failed");
    let market_id: i32 = market_row.get("id");

    // Insert a pool with status 'Locked' (as &str, cast to pool_status)
    let row = sqlx::query("INSERT INTO pool (market_id, name, type, status) VALUES ($1, $2, $3, $4::pool_status) RETURNING id")
        .bind(market_id)
        .bind("Test Pool")
        .bind(1)
        .bind("Locked")
        .fetch_one(&pool)
        .await
        .expect("Insert failed");

    let id: i32 = row.get("id");

    // Query the pool and check status mapping
    let pool_obj: Pool = sqlx::query_as::<_, Pool>("SELECT * FROM pool WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("Fetch failed");

    assert_eq!(pool_obj.status, PoolStatus::Locked);

    // Clean up
    sqlx::query("DELETE FROM pool WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("Cleanup failed");
    sqlx::query("DELETE FROM market WHERE id = $1")
        .bind(market_id)
        .execute(&pool)
        .await
        .expect("Cleanup market failed");
}
