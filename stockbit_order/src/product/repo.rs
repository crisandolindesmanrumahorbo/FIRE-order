use anyhow::Result;
use sqlx::Postgres;

use super::model::Product;

#[derive(Clone)]
pub struct ProductRepository {
    pub pool: sqlx::Pool<Postgres>,
}

impl ProductRepository {
    pub fn new(pool: sqlx::Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn get_product_by_symbol(&self, symbol: &str) -> Result<Product> {
        Ok(sqlx::query_as::<_, Product>(
            "SELECT product_id, symbol, name FROM products WHERE symbol = $1",
        )
        .bind(symbol)
        .fetch_one(&self.pool)
        .await?)
    }
}
