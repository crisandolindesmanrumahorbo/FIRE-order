use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/* TODO product save in redis*/
#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct Order {
    pub order_id: Option<i32>,
    pub product_symbol: String,
    pub product_name: String,
    pub side: char,
    pub price: i32,
    pub lot: i32,
    pub expiry: Expiry,
    pub created_at: DateTime<Utc>,
    pub user_id: i32,
    pub product_id: i32,
}

impl Order {
    pub fn new(
        order_form: &OrderForm,
        user_id: i32,
        product_id: i32,
        product_name: &str,
    ) -> Result<Order, anyhow::Error> {
        Ok(Self {
            order_id: None,
            product_symbol: order_form.symbol.to_string(),
            product_name: product_name.to_string(),
            side: order_form.side,
            price: order_form.price as i32,
            lot: order_form.lot as i32,
            expiry: order_form.expiry.as_str().try_into()?,
            created_at: Utc::now(),
            user_id,
            product_id,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct OrderForm {
    pub symbol: String,
    pub side: char,
    pub price: u32,
    pub lot: u32,
    pub expiry: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OrderFormServer {
    pub symbol: String,
    pub side: char,
    pub price: u32,
    pub lot: u32,
    pub expiry: String,
    pub user_id: u32,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Orders {
    #[sqlx(rename = "product_symbol")]
    pub symbol: String,
    #[sqlx(rename = "product_name")]
    pub name: String,
    pub side: String,
    pub price: i32,
    pub lot: i32,
    pub expiry: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum Expiry {
    GTC,
    GFD,
}

impl TryFrom<&str> for Expiry {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, anyhow::Error> {
        match value {
            "GTC" => Ok(Expiry::GTC),
            "GFD" => Ok(Expiry::GTC),
            _ => Err(anyhow::anyhow!("Expiry not found")),
        }
    }
}

impl ToString for Expiry {
    fn to_string(&self) -> String {
        match self {
            Expiry::GTC => "GTC".to_string(),
            Expiry::GFD => "GFD".to_string(),
        }
    }
}
