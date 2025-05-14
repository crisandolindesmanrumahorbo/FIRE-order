use anyhow::Result;
use auth_validate::jwt::verify_jwt;
use request_http_parser::parser::Request;
use rust_decimal::Decimal;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use tracing::info;

use crate::{
    account::{model::GetAccount, repo::AccountRepo},
    cfg::CONFIG,
    constant::{OK_RESPONSE, UNAUTHORIZED},
    order::{
        model::{Order, OrderForm, Orders},
        repo::OrderRepo,
    },
    portfolio::{
        model::{GetPortfolio, Portfolio},
        repo::PortoRepo,
    },
    product::repo::ProductRepository,
    utils::{self, extract_token, ser_to_str},
};

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
}

impl Service {
    pub fn new(
        product_repo: ProductRepository,
        order_repo: OrderRepo,
        account_repo: AccountRepo,
        porto_repo: PortoRepo,
    ) -> Self {
        Self {
            product_repo,
            order_repo,
            account_repo,
            porto_repo,
        }
    }

    pub async fn create_order(&self, message: &str, stream: &mut TcpStream, user_id_str: &str) {
        let user_id = user_id_str.parse::<i32>().expect("user id parse error");
        match utils::des_from_str::<OrderForm>(message) {
            Ok(order_form) => {
                let product = self
                    .product_repo
                    .get_product_by_symbol(&order_form.symbol)
                    .await
                    .expect("query error");
                let order = Order::new(&order_form, user_id, product.product_id, &product.name)
                    .expect("parse order");
                info!("{:?}", order);

                let account = match self.account_repo.get_account_by_user_id(user_id).await {
                    Ok(account) => account,
                    Err(_) => {
                        let response = Response {
                            status: String::from("error"),
                            message: chrono::Utc::now().to_string(),
                        };
                        let response_json =
                            ser_to_str(&response).expect("Error serialize response");
                        let frame: Vec<u8> = utils::create_websocket_frame(&response_json);
                        stream.write_all(&frame).await.expect("err write response");
                        return;
                    }
                };
                let exist_porto = match self.porto_repo.get_by_symbol(&order_form.symbol).await {
                    Ok(porto) => Some(porto),
                    Err(e) => match e {
                        sqlx::Error::RowNotFound => None,
                        e => {
                            println!("{:?}", e);
                            let response = Response {
                                status: String::from("error"),
                                message: chrono::Utc::now().to_string(),
                            };
                            let response_json =
                                ser_to_str(&response).expect("Error serialize response");
                            let frame: Vec<u8> = utils::create_websocket_frame(&response_json);
                            stream.write_all(&frame).await.expect("err write response");
                            return;
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
                let frame: Vec<u8> = utils::create_websocket_frame(&response_json);
                stream.write_all(&frame).await.expect("err write response")
            }
            Err(_) => {
                let response = Response {
                    status: String::from("error"),
                    message: chrono::Utc::now().to_string(),
                };
                let response_json = ser_to_str(&response).expect("Error serialize response");

                let frame: Vec<u8> = utils::create_websocket_frame(&response_json);
                stream.write_all(&frame).await.expect("err write response")
            }
        };
    }

    pub async fn get_orders(&self, request: Request, stream: &mut TcpStream) -> Result<()> {
        let token = match extract_token(&request.headers) {
            Some(token) => token,
            None => {
                info!("error extract_token");
                stream
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
        let user_id = match verify_jwt(&token, &CONFIG.jwt_public_key) {
            Ok(user_id) => user_id,
            Err(_) => {
                info!("error verify_jwt");
                stream
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

        let orders: Vec<Orders> = match self.order_repo.get_all_by_user_id(&user_id).await {
            Ok(orders) => orders,
            Err(e) => {
                info!("error {}", e);
                stream
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
        stream
            .write_all(format!("{}{}", OK_RESPONSE.to_string(), response_json).as_bytes())
            .await?;

        Ok(())
    }
}
