use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Market {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i32>,
    pub image_url: Option<String>,
    pub event_source_url: Option<String>,
    pub start_time: Option<i64>,
    pub lock_time: Option<i64>,
    pub end_time: Option<i64>,
    pub option1: Option<String>,
    pub option2: Option<String>,
    pub min_bet_amount: Option<BigDecimal>,
    pub max_bet_amount: Option<BigDecimal>,
    pub creator_fee: Option<i16>,
    pub is_private: Option<bool>,
    pub creator_address: Option<String>,
    pub created_timestamp: Option<i64>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewMarket {
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i32>,
    pub image_url: Option<String>,
    pub event_source_url: Option<String>,
    pub start_time: Option<i64>,
    pub lock_time: Option<i64>,
    pub end_time: Option<i64>,
    pub option1: Option<String>,
    pub option2: Option<String>,
    pub min_bet_amount: Option<BigDecimal>,
    pub max_bet_amount: Option<BigDecimal>,
    pub creator_fee: Option<i16>,
    pub is_private: Option<bool>,
    pub creator_address: Option<String>,
    pub created_timestamp: Option<i64>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MarketCategory {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: i32,
    pub name: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MarketTag {
    pub id: i32,
    pub market_id: i32,
    pub tag_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketWithTags {
    pub market: Market,
    pub tags: Vec<Tag>,
}
