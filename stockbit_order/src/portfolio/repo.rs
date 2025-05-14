use super::model::{GetPortfolio, Portfolio};
use anyhow::Result;
use sqlx::Postgres;

#[derive(Clone)]
pub struct PortoRepo {
    pub pool: sqlx::Pool<Postgres>,
}

impl PortoRepo {
    pub fn new(pool: sqlx::Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, porto: &Portfolio) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            r#"INSERT INTO portfolios (user_id, product_name, product_symbol, 
                invested_value, lot, avg_price, product_id)
                VALUES ($1, $2, $3, $4, $5, $6, $7) 
                RETURNING portfolio_id"#,
        )
        .bind(&porto.user_id)
        .bind(&porto.product_name)
        .bind(&porto.product_symbol)
        .bind(&porto.invested_value)
        .bind(&porto.lot)
        .bind(&porto.avg_price)
        .bind(&porto.product_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    pub async fn get_by_symbol(&self, symbol: &str) -> Result<GetPortfolio, sqlx::Error> {
        sqlx::query_as::<_, GetPortfolio>(
            r#"SELECT portfolio_id, lot, invested_value, avg_price FROM portfolios WHERE product_symbol = $1"#,
        )
        .bind(symbol)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update(&self, new_porto: GetPortfolio) -> Result<i32, sqlx::Error> {
        let row: (i32,) = sqlx::query_as(
            r#"
            UPDATE portfolios
            SET lot = $1, invested_value = $2, avg_price = $3
            WHERE portfolio_id = $4
            RETURNING portfolio_id"#,
        )
        .bind(&new_porto.lot)
        .bind(&new_porto.invested_value)
        .bind(&new_porto.avg_price)
        .bind(&new_porto.portfolio_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }
}
