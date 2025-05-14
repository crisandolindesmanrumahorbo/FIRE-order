use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Account {
    pub account_id: Option<i32>,
    pub user_id: i32,
    pub balance: i64,
    pub invested_value: i64,
}

impl Account {
    pub fn new(user_id: i32) -> Self {
        Self {
            account_id: None,
            user_id,
            balance: 0,
            invested_value: 0,
        }
    }
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct GetAccount {
    pub account_id: i32,
    pub balance: i64,
    pub invested_value: i64,
}

impl GetAccount {
    pub fn new(balance: i64, invested_value: i64, account_id: i32) -> Self {
        Self {
            account_id,
            balance,
            invested_value,
        }
    }
}
