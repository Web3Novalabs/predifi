use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Market {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MarketCategory {
    pub id: i32,
    pub name: String,
}
