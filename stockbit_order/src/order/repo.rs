use super::model::{Order, Orders};
use anyhow::{Ok, Result};
use sqlx::Postgres;

#[derive(Clone)]
pub struct OrderRepo {
    pub pool: sqlx::Pool<Postgres>,
}

impl OrderRepo {
    pub fn new(pool: sqlx::Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, order: &Order) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            r#"INSERT INTO orders (product_symbol, product_name, side, 
                price, lot, expiry, created_at, user_id, product_id)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) 
                RETURNING order_id"#,
        )
        .bind(&order.product_symbol)
        .bind(&order.product_name)
        .bind(&order.side.to_string())
        .bind(&order.price)
        .bind(&order.lot)
        .bind(&order.expiry.to_string())
        .bind(&order.created_at)
        .bind(&order.user_id)
        .bind(&order.product_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0 as i32)
    }

    pub async fn get_all_by_user_id(&self, user_id: &str) -> Result<Vec<Orders>> {
        // TODO
        // price in database is decimal but in our rust its i32, consider 1 type
        let orders = sqlx::query_as::<_, Orders>(
            r#"SELECT product_symbol, product_name, side, price::integer as price,
                lot, expiry, created_at FROM orders WHERE user_id = $1"#,
        )
        .bind(user_id.parse::<i32>().expect("error parse user_id"))
        .fetch_all(&self.pool)
        .await?;
        Ok(orders)
    }
}
