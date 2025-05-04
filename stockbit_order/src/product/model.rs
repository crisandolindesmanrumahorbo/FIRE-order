use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct Product {
    pub product_id: i32,
    pub name: String,
    pub symbol: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct ProductDetail {
    pub product_id: i32,
    pub name: String,
    pub symbol: String,
    pub tags: String,
    pub last_updated: DateTime<Utc>,
}
