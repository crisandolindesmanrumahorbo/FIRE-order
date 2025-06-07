use super::model::{GetPortfolio, Portfolio, Portfolios};
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
        println!("{:?}", porto);
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
        .await
        .expect("error insert porto cukkk");
        Ok(row.0)
    }

    pub async fn get_by_symbol(
        &self,
        symbol: &str,
        user_id: i32,
    ) -> Result<GetPortfolio, sqlx::Error> {
        sqlx::query_as::<_, GetPortfolio>(
            r#"SELECT portfolio_id, lot, invested_value, avg_price FROM portfolios WHERE product_symbol = $1 AND user_id = $2"#,
        )
        .bind(symbol)
            .bind(user_id)
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
        .await
        .expect("error update");
        Ok(row.0)
    }

    pub async fn get_all_by_user_id(&self, user_id: i32) -> Result<Vec<Portfolios>> {
        let portfolios = sqlx::query_as::<_, Portfolios>(
            r#"SELECT lot, invested_value, avg_price, product_name, product_symbol 
            FROM portfolios WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(portfolios)
    }
}
