use crate::models::pool::{NewPool, Pool, UserPool};
use bigdecimal::BigDecimal;
use sqlx::{Pool as SqlxPool, Postgres};

pub async fn create_pool(
    pool: &SqlxPool<Postgres>,
    new_pool: &NewPool,
) -> Result<Pool, sqlx::Error> {
    sqlx::query_as::<_, Pool>(
        "INSERT INTO pool (market_id, name, type, description, image_url, event_source_url, start_time, lock_time, end_time, option1, option2, min_bet_amount, max_bet_amount, creator_fee, is_private, category_id) VALUES \
        ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16) RETURNING *"
    )
    .bind(new_pool.market_id)
    .bind(&new_pool.name)
    .bind(new_pool.r#type)
    .bind(&new_pool.description)
    .bind(&new_pool.image_url)
    .bind(&new_pool.event_source_url)
    .bind(new_pool.start_time)
    .bind(new_pool.lock_time)
    .bind(new_pool.end_time)
    .bind(&new_pool.option1)
    .bind(&new_pool.option2)
    .bind(&new_pool.min_bet_amount)
    .bind(&new_pool.max_bet_amount)
    .bind(new_pool.creator_fee)
    .bind(new_pool.is_private)
    .bind(new_pool.category_id)
    .fetch_one(pool)
    .await
}

pub async fn get_pool(pool: &SqlxPool<Postgres>, id: i32) -> Result<Pool, sqlx::Error> {
    sqlx::query_as::<_, Pool>("SELECT * FROM pool WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn create_user_pool(
    pool: &SqlxPool<Postgres>,
    user_id: &str,
    pool_id: i32,
    amount_staked: &BigDecimal,
) -> Result<UserPool, sqlx::Error> {
    sqlx::query_as::<_, UserPool>(
        "INSERT INTO user_pool (user_id, pool_id, amount_staked) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(user_id)
    .bind(pool_id)
    .bind(amount_staked)
    .fetch_one(pool)
    .await
}

pub async fn get_user_pool(pool: &SqlxPool<Postgres>, id: i32) -> Result<UserPool, sqlx::Error> {
    sqlx::query_as::<_, UserPool>("SELECT * FROM user_pool WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}
