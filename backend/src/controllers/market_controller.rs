use crate::models::market::{Market, MarketCategory};
use sqlx::{Pool, Postgres};

pub async fn create_market(
    pool: &Pool<Postgres>,
    name: &str,
    description: Option<&str>,
    category_id: Option<i32>,
) -> Result<Market, sqlx::Error> {
    sqlx::query_as::<_, Market>(
        "INSERT INTO market (name, description, category_id) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(name)
    .bind(description)
    .bind(category_id)
    .fetch_one(pool)
    .await
}

pub async fn get_market(pool: &Pool<Postgres>, id: i32) -> Result<Market, sqlx::Error> {
    sqlx::query_as::<_, Market>("SELECT * FROM market WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn create_market_category(
    pool: &Pool<Postgres>,
    name: &str,
) -> Result<MarketCategory, sqlx::Error> {
    sqlx::query_as::<_, MarketCategory>(
        "INSERT INTO market_category (name) VALUES ($1) RETURNING *",
    )
    .bind(name)
    .fetch_one(pool)
    .await
}

pub async fn get_market_category(
    pool: &Pool<Postgres>,
    id: i32,
) -> Result<MarketCategory, sqlx::Error> {
    sqlx::query_as::<_, MarketCategory>("SELECT * FROM market_category WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}
