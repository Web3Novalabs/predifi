use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Debug, Serialize, Deserialize)]
pub struct NewPool {
    pub market_id: i32,
    pub name: String,
    pub r#type: i16,
    pub description: Option<String>,
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
    pub category_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Type, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "pool_status")]
pub enum PoolStatus {
    Active,
    Locked,
    Settled,
    Closed,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Pool {
    pub id: i32,
    pub market_id: i32,
    pub name: String,
    pub r#type: i16,
    pub description: Option<String>,
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
    pub category_id: Option<i32>,
    pub status: PoolStatus,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserPool {
    pub id: i32,
    pub user_id: String,
    pub pool_id: i32,
    pub amount_staked: BigDecimal,
}
