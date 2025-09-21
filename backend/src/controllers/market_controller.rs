use crate::models::market::{Market, MarketCategory, MarketTag, MarketWithTags, NewMarket, Tag};
use sqlx::{Pool, Postgres};

#[allow(dead_code)]
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

pub async fn create_market_with_tags(
    pool: &Pool<Postgres>,
    new_market: &NewMarket,
) -> Result<MarketWithTags, sqlx::Error> {
    let mut transaction = pool.begin().await?;

    // Insert market with all fields from Cairo contract
    let market = sqlx::query_as::<_, Market>(
        r#"
        INSERT INTO market (
            name, description, category_id, image_url, event_source_url,
            start_time, lock_time, end_time, option1, option2,
            min_bet_amount, max_bet_amount, creator_fee, is_private,
            creator_address, created_timestamp, status
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        RETURNING *
        "#,
    )
    .bind(&new_market.name)
    .bind(&new_market.description)
    .bind(new_market.category_id)
    .bind(&new_market.image_url)
    .bind(&new_market.event_source_url)
    .bind(new_market.start_time)
    .bind(new_market.lock_time)
    .bind(new_market.end_time)
    .bind(&new_market.option1)
    .bind(&new_market.option2)
    .bind(&new_market.min_bet_amount)
    .bind(&new_market.max_bet_amount)
    .bind(new_market.creator_fee)
    .bind(new_market.is_private)
    .bind(&new_market.creator_address)
    .bind(new_market.created_timestamp)
    .bind(Some("active"))
    .fetch_one(&mut *transaction)
    .await?;

    let mut tags = Vec::new();

    // Handle tags if provided
    if let Some(tag_names) = &new_market.tags {
        for tag_name in tag_names {
            // Upsert tag (create if doesn't exist, get if exists)
            let tag = sqlx::query_as::<_, Tag>(
                r#"
                INSERT INTO tags (name) VALUES ($1)
                ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
                RETURNING *
                "#,
            )
            .bind(tag_name)
            .fetch_one(&mut *transaction)
            .await?;

            // Create market-tag association
            sqlx::query_as::<_, MarketTag>(
                r#"
                INSERT INTO market_tags (market_id, tag_id) VALUES ($1, $2)
                ON CONFLICT (market_id, tag_id) DO NOTHING
                RETURNING *
                "#,
            )
            .bind(market.id)
            .bind(tag.id)
            .fetch_optional(&mut *transaction)
            .await?;

            tags.push(tag);
        }
    }

    transaction.commit().await?;

    Ok(MarketWithTags { market, tags })
}

#[allow(dead_code)]
pub async fn get_market(pool: &Pool<Postgres>, id: i32) -> Result<Market, sqlx::Error> {
    sqlx::query_as::<_, Market>("SELECT * FROM market WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}

pub async fn get_market_with_tags(
    pool: &Pool<Postgres>,
    id: i32,
) -> Result<MarketWithTags, sqlx::Error> {
    let market = get_market(pool, id).await?;

    let tags = sqlx::query_as::<_, Tag>(
        r#"
        SELECT t.* FROM tags t
        INNER JOIN market_tags mt ON t.id = mt.tag_id
        WHERE mt.market_id = $1
        "#,
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(MarketWithTags { market, tags })
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub async fn get_market_category(
    pool: &Pool<Postgres>,
    id: i32,
) -> Result<MarketCategory, sqlx::Error> {
    sqlx::query_as::<_, MarketCategory>("SELECT * FROM market_category WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
}

#[allow(dead_code)]
pub async fn get_all_tags(pool: &Pool<Postgres>) -> Result<Vec<Tag>, sqlx::Error> {
    sqlx::query_as::<_, Tag>("SELECT * FROM tags ORDER BY name")
        .fetch_all(pool)
        .await
}

#[allow(dead_code)]
pub async fn get_tags_by_market_id(
    pool: &Pool<Postgres>,
    market_id: i32,
) -> Result<Vec<Tag>, sqlx::Error> {
    sqlx::query_as::<_, Tag>(
        r#"
        SELECT t.* FROM tags t
        INNER JOIN market_tags mt ON t.id = mt.tag_id
        WHERE mt.market_id = $1
        ORDER BY t.name
        "#,
    )
    .bind(market_id)
    .fetch_all(pool)
    .await
}
