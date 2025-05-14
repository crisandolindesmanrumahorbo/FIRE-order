use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct Portfolio {
    pub portfolio_id: Option<i32>,
    pub user_id: i32,
    pub product_id: i32,
    pub product_name: String,
    pub product_symbol: String,
    pub lot: i32,
    pub invested_value: i64,
    pub avg_price: Decimal,
}

impl Portfolio {
    pub fn new(
        user_id: i32,
        product_id: i32,
        product_name: String,
        product_symbol: String,
        lot: i32,
        invested_value: i64,
        avg_price: Decimal,
    ) -> Self {
        Self {
            portfolio_id: None,
            user_id,
            product_id,
            product_name,
            product_symbol,
            lot,
            invested_value,
            avg_price,
        }
    }
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct GetPortfolio {
    pub portfolio_id: i32,
    pub lot: i32,
    pub invested_value: i64,
    pub avg_price: Decimal,
}

impl GetPortfolio {
    pub fn new(portfolio_id: i32, lot: i32, invested_value: i64, avg_price: Decimal) -> Self {
        Self {
            portfolio_id,
            lot,
            invested_value,
            avg_price,
        }
    }
}
