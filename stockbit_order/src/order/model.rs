use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/* TODO product save in redis*/
#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct Order {
    pub order_id: Option<i32>,
    pub product_symbol: String,
    pub product_name: String,
    pub side: char,
    pub price: u32,
    pub lot: u32,
    pub expiry: Expiry,
    pub created_at: DateTime<Utc>,
    pub user_id: i32,
    pub product_id: i32,
}

impl Order {
    pub fn new(
        order_form: OrderForm,
        user_id: &str,
        product_id: i32,
        product_name: String,
    ) -> Result<Order, anyhow::Error> {
        Ok(Self {
            order_id: None,
            product_symbol: order_form.symbol,
            product_name,
            side: order_form.side,
            price: order_form.price,
            lot: order_form.lot,
            expiry: order_form.expiry.as_str().try_into()?,
            created_at: Utc::now(),
            user_id: user_id.parse::<i32>().unwrap(),
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
