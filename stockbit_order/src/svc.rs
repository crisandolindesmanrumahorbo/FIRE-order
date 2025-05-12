use anyhow::Result;
use auth_validate::jwt::verify_jwt;
use request_http_parser::parser::Request;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use tracing::info;

use crate::{
    cfg::CONFIG,
    constant::{BAD_REQUEST, OK_RESPONSE, UNAUTHORIZED},
    order::{
        model::{Order, OrderForm, Orders},
        repo::OrderRepo,
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
}

impl Service {
    pub fn new(product_repo: ProductRepository, order_repo: OrderRepo) -> Self {
        Self {
            product_repo,
            order_repo,
        }
    }

    pub async fn create_order(&self, message: &str, stream: &mut TcpStream, user_id: &str) {
        match utils::des_from_str::<OrderForm>(message) {
            Ok(order_form) => {
                let product = self
                    .product_repo
                    .get_product_by_symbol(&order_form.symbol)
                    .await
                    .expect("query error");
                let order = Order::new(order_form, &user_id, product.product_id, product.name)
                    .expect("parse order");
                // send kafka
                let order_id = self.order_repo.insert(&order).await.expect("error insert");
                info!("{:?}", order);
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
