use anyhow::Result;
use sqlx::Postgres;

use super::model::GetAccount;

#[derive(Clone)]
pub struct AccountRepo {
    pub pool: sqlx::Pool<Postgres>,
}

impl AccountRepo {
    pub fn new(pool: sqlx::Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn get_account_by_user_id(&self, user_id: i32) -> Result<GetAccount, sqlx::Error> {
        sqlx::query_as::<_, GetAccount>(
            r#"SELECT balance, invested_value, account_id FROM accounts WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_account(&self, account: &GetAccount) -> Result<i32, sqlx::Error> {
        let row: (i32,) = sqlx::query_as(
            r#"
            UPDATE accounts
            SET balance = $1, invested_value = $2 
            WHERE account_id = $3
            RETURNING account_id"#,
        )
        .bind(&account.balance)
        .bind(&account.invested_value)
        .bind(&account.account_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }
}
