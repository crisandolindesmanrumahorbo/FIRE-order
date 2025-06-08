use crate::constant::{BAD_REQUEST, INTERNAL_ERROR};
use crate::error::OrderError;
use crate::order::model::OrderFormServer;
use crate::product::model::Product;
use crate::redis::RedisCache;
use crate::{
    account::{
        model::{GetAccount, GetAccountDTO},
        repo::AccountRepo,
    },
    constant::{OK_RESPONSE, UNAUTHORIZED},
    order::{
        model::{Order, OrderForm, Orders},
        repo::OrderRepo,
    },
    portfolio::{
        model::{GetPortfolio, Portfolio, Portfolios},
        repo::PortoRepo,
    },
    product::repo::ProductRepository,
    utils::{self, ser_to_str},
};
use anyhow::Result;
use request_http_parser::parser::Request;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tracing::info;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Response<T> {
    pub status: String,
    pub message: T,
}

#[derive(Clone)]
pub struct Service {
    product_repo: ProductRepository,
    order_repo: OrderRepo,
    account_repo: AccountRepo,
    porto_repo: PortoRepo,
    redis_cache: Arc<Mutex<RedisCache>>,
}

impl Service {
    pub fn new(
        product_repo: ProductRepository,
        order_repo: OrderRepo,
        account_repo: AccountRepo,
        porto_repo: PortoRepo,
        redis_cache: RedisCache,
    ) -> Self {
        Self {
            product_repo,
            order_repo,
            account_repo,
            porto_repo,
            redis_cache: Arc::new(Mutex::new(redis_cache)),
        }
    }
    pub async fn create_order(&self, message: &str, stream: &mut TcpStream, user_id: i32) {
        let order_form = match utils::des_from_str::<OrderForm>(message) {
            Ok(order_form) => order_form,
            Err(_) => {
                let response = Response {
                    status: String::from("error"),
                    message: chrono::Utc::now().to_string(),
                };
                let response_json = ser_to_str(&response).expect("Error serialize response");

                let frame: Vec<u8> = utils::create_websocket_frame(&response_json);
                stream.write_all(&frame).await.expect("err write response");
                return;
            }
        };

        match self.handle_order(order_form, user_id).await {
            Ok(res) => {
                let frame: Vec<u8> = utils::create_websocket_frame(&res);
                stream.write_all(&frame).await.expect("err write response");
            }
            Err(_) => {
                let response = Response {
                    status: String::from("error"),
                    message: chrono::Utc::now().to_string(),
                };
                let response_json = ser_to_str(&response).expect("Error serialize response");

                let frame: Vec<u8> = utils::create_websocket_frame(&response_json);
                stream.write_all(&frame).await.expect("err write response");
            }
        }
    }

    pub async fn get_orders(
        &self,
        user_id: i32,
        mut writer: impl AsyncWrite + Unpin,
    ) -> Result<()> {
        let orders: Vec<Orders> = match self.order_repo.get_all_by_user_id(user_id).await {
            Ok(orders) => orders,
            Err(e) => {
                info!("error {}", e);
                writer
                    .write_all(
                        format!(
                            "{}{}",
                            UNAUTHORIZED.to_string(),
                            "401 unathorized".to_string()
                        )
                        .as_bytes(),
                    )
                    .await?;
                return Ok(());
            }
        };
        let response = Response {
            status: String::from("ok"),
            message: orders,
        };
        let response_json = ser_to_str(&response).expect("Error serialize response");
        writer
            .write_all(format!("{}{}", OK_RESPONSE.to_string(), response_json).as_bytes())
            .await?;

        Ok(())
    }

    pub async fn get_portfolios(
        &self,
        _request: Request,
        user_id: i32,
        mut writer: impl AsyncWrite + Unpin,
    ) -> Result<()> {
        let portfolios: Vec<Portfolios> = match self.porto_repo.get_all_by_user_id(user_id).await {
            Ok(portfolios) => portfolios,
            Err(e) => {
                info!("error {}", e);
                writer
                    .write_all(
                        format!(
                            "{}{}",
                            UNAUTHORIZED.to_string(),
                            "401 unathorized".to_string()
                        )
                        .as_bytes(),
                    )
                    .await?;
                return Ok(());
            }
        };
        let response = Response {
            status: String::from("ok"),
            message: portfolios,
        };
        let response_json = ser_to_str(&response).expect("Error serialize response");
        writer
            .write_all(format!("{}{}", OK_RESPONSE.to_string(), response_json).as_bytes())
            .await?;

        Ok(())
    }

    pub async fn get_account(
        &self,
        _request: Request,
        user_id: i32,
        mut writer: impl AsyncWrite + Unpin,
    ) -> Result<()> {
        let account = match self.account_repo.get_account_by_user_id(user_id).await {
            Ok(account) => GetAccountDTO {
                balance: account.balance,
                invested_value: account.invested_value,
            },
            Err(e) => {
                info!("error {}", e);
                writer
                    .write_all(
                        format!(
                            "{}{}",
                            UNAUTHORIZED.to_string(),
                            "401 unathorized".to_string()
                        )
                        .as_bytes(),
                    )
                    .await?;
                return Ok(());
            }
        };
        let response = Response {
            status: String::from("ok"),
            message: account,
        };
        let response_json = ser_to_str(&response).expect("Error serialize response");
        writer
            .write_all(format!("{}{}", OK_RESPONSE.to_string(), response_json).as_bytes())
            .await?;

        Ok(())
    }

    pub async fn create_order_nonws(
        &self,
        request: Request,
        mut writer: impl AsyncWrite + Unpin,
    ) -> Result<()> {
        let body = match request.body {
            Some(body) => body,
            None => {
                writer
                    .write_all(format!("{}{}", BAD_REQUEST.to_string(), "".to_string()).as_bytes())
                    .await?;
                return Ok(());
            }
        };
        let order_form_server = match utils::des_from_str::<OrderFormServer>(&body) {
            Ok(order_form) => order_form,
            Err(_) => {
                writer
                    .write_all(format!("{}{}", BAD_REQUEST.to_string(), "".to_string()).as_bytes())
                    .await?;
                return Ok(());
            }
        };
        let order_form = OrderForm {
            symbol: order_form_server.symbol,
            side: order_form_server.side,
            price: order_form_server.price,
            lot: order_form_server.lot,
            expiry: order_form_server.expiry,
        };
        let user_id = order_form_server.user_id as i32;
        match self.handle_order(order_form, user_id).await {
            Ok(res) => {
                writer
                    .write_all(format!("{}{}", OK_RESPONSE.to_string(), res).as_bytes())
                    .await?;
            }
            Err(why) => match why {
                OrderError::BadRequest => {
                    writer
                        .write_all(
                            format!("{}{}", BAD_REQUEST.to_string(), "".to_string()).as_bytes(),
                        )
                        .await?;
                }
                OrderError::Serde => {
                    writer
                        .write_all(
                            format!("{}{}", BAD_REQUEST.to_string(), "".to_string()).as_bytes(),
                        )
                        .await?;
                }
                OrderError::Redis => {
                    writer
                        .write_all(
                            format!("{}{}", INTERNAL_ERROR.to_string(), "".to_string()).as_bytes(),
                        )
                        .await?;
                }
                OrderError::Database => {
                    writer
                        .write_all(
                            format!("{}{}", INTERNAL_ERROR.to_string(), "".to_string()).as_bytes(),
                        )
                        .await?;
                }
            },
        }
        Ok(())
    }

    pub async fn handle_order(
        &self,
        order_form: OrderForm,
        user_id: i32,
    ) -> Result<String, OrderError> {
        let mut format = format!("product:{}", &order_form.symbol);
        let mut cache = self.redis_cache.lock().await;
        let product = match cache.get_cached(&format).await {
            Ok(product) => match product {
                Some(product) => {
                    info!("Hit cache {}", &format);
                    product
                }
                None => {
                    let product = self
                        .product_repo
                        .get_product_by_symbol(&order_form.symbol)
                        .await
                        .expect("query product error");
                    let _ = cache.set_cache::<Product>(&format, &product).await;
                    product
                }
            },
            Err(_) => {
                return Err(OrderError::Redis);
            }
        };
        let order = Order::new(&order_form, user_id, product.product_id, &product.name)
            .expect("parse order");
        info!("{:?}", order);

        format = format!("account:{}", user_id);
        let account = match cache.get_cached(&format).await {
            Ok(account) => match account {
                Some(account) => {
                    info!("Hit cache {}", &format);
                    account
                }
                None => {
                    let account = self
                        .account_repo
                        .get_account_by_user_id(user_id as i32)
                        .await
                        .expect("query account error");
                    let _ = cache.set_cache::<GetAccount>(&format, &account).await;
                    account
                }
            },
            Err(_) => {
                return Err(OrderError::Redis);
            }
        };
        let exist_porto = match self
            .porto_repo
            .get_by_symbol(&order_form.symbol, user_id)
            .await
        {
            Ok(porto) => Some(porto),
            Err(e) => match e {
                sqlx::Error::RowNotFound => None,
                e => {
                    println!("{:?}", e);
                    return Err(OrderError::Database);
                }
            },
        };
        let total_order_form = order_form.price * (order_form.lot * 100);
        match exist_porto {
            Some(porto) => {
                let new_lot = porto.lot + order_form.lot as i32;
                let new_invested_port = porto.invested_value + total_order_form as i64;
                let order_price: Decimal = order_form.price.into();
                let order_lot: Decimal = order_form.lot.into();
                let current_lot: Decimal = porto.lot.into();
                let new_lot_dec: Decimal = new_lot.into();
                let order_value = order_price * order_lot;
                let existing_value = porto.avg_price * current_lot;
                let total_value = order_value + existing_value;
                let new_avg_price = total_value / new_lot_dec;

                self.porto_repo
                    .update(GetPortfolio::new(
                        porto.portfolio_id,
                        new_lot,
                        new_invested_port,
                        new_avg_price,
                    ))
                    .await
                    .expect("error update porto");
            }
            None => {
                let new_avg_price: Decimal = order_form.price.into();
                let new = Portfolio::new(
                    user_id,
                    product.product_id,
                    product.name,
                    product.symbol,
                    order_form.lot as i32,
                    total_order_form as i64,
                    new_avg_price,
                );
                self.porto_repo
                    .insert(&new)
                    .await
                    .expect("error insert porto");
            }
        }
        // send kafka -> prevent error when do order
        let order_id = self.order_repo.insert(&order).await.expect("error insert");
        let total = order_form.price * (order_form.lot * 100);
        let new_balance = &account.balance - total as i64;
        let new_invested = &account.invested_value + total as i64;
        let updated = GetAccount::new(new_balance, new_invested, account.account_id);
        self.account_repo
            .update_account(&updated)
            .await
            .expect("failed update account");
        let response = Response {
            status: String::from("ok"),
            message: order_id.to_string(),
        };
        let response_json = ser_to_str(&response).expect("Error serialize response");
        Ok(response_json)
    }
}
