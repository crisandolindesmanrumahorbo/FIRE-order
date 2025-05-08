use crate::cfg::CONFIG;
use crate::constant::{LOGGING_HANDSHAKE, LOGGING_MESSAGE, UNAUTHORIZED};
use crate::logging::thread_logging;
use crate::order::model::{Order, OrderForm};
use crate::order::repo::OrderRepo;
use crate::product::repo::ProductRepository;
use crate::utils;
use crate::utils::ser_to_str;
use auth_validate::jwt::verify_jwt;
use request_http_parser::parser::Request;
use std::error::Error;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::info;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Response {
    pub status: String,
    pub message: String,
}

pub async fn handle_websocket(
    request: Request,
    product_repo: &Arc<ProductRepository>,
    order_repo: &Arc<OrderRepo>,
    stream: &mut TcpStream,
) -> Result<(), Box<dyn Error>> {
    if let Some(user_id) = verify_token(&request) {
        // handshake success
        let sec_websocket_key = request.headers.get("sec-websocket-key").map_or("", |v| v);
        let sec_websocket_accept = utils::generate_accept_key(sec_websocket_key);
        // Send WebSocket handshake response
        let response = format!(
            "HTTP/1.1 101 Switching Protocols\r\n\
                    Upgrade: websocket\r\n\
                    Connection: Upgrade\r\n\
                    Sec-WebSocket-Accept: {}\r\n\
                    \r\n",
            sec_websocket_accept
        );
        stream.write_all(response.as_bytes()).await?;
        stream.flush().await?;

        thread_logging(LOGGING_HANDSHAKE);
        // Start handling WebSocket messages
        handle_message(stream, user_id, product_repo, order_repo).await;

        info!("Closing connection...");
        let _ = stream.shutdown().await;
        Ok(())
    } else {
        info!("WebSocket handshake failed!");
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
        Ok(())
    }
}

fn verify_token(request: &Request) -> Option<String> {
    info!("Client is requesting WebSocket connection");
    let token = match &request.params {
        Some(params) => match params.get("token") {
            Some(token) => token.to_string(),
            None => {
                info!("Token from params Error");
                return None;
            }
        },
        None => {
            info!("Params Error");
            return None;
        }
    };
    match verify_jwt(&token, &CONFIG.jwt_public_key) {
        Ok(user_id) => Some(user_id),
        Err(err) => {
            info!("Verification failed: {}", err);
            None
        }
    }
}

async fn handle_message(
    stream: &mut TcpStream,
    user_id: String,
    product_repo: &Arc<ProductRepository>,
    order_repo: &Arc<OrderRepo>,
) {
    loop {
        thread_logging(LOGGING_MESSAGE);
        let mut buffer = [0; 1024];
        if let Ok(bytes_read) = stream.read(&mut buffer).await {
            if bytes_read == 0 {
                info!("Client disconnected");
                break;
            }

            if let Some(message) = utils::parse_websocket_framev2(&buffer[..bytes_read]) {
                info!("Received WebSocket message: {}", message);
                match utils::des_from_str::<OrderForm>(&message) {
                    Ok(order_form) => {
                        let product = product_repo
                            .get_product_by_symbol(&order_form.symbol)
                            .await
                            .expect("query error");
                        let order =
                            Order::new(order_form, &user_id, product.product_id, product.name)
                                .expect("parse order");
                        // send kafka
                        let order_id = order_repo.insert(&order).await.expect("error insert");
                        info!("{:?}", order);
                        let response = Response {
                            status: String::from("ok"),
                            message: order_id.to_string(),
                        };
                        let response_json =
                            ser_to_str(&response).expect("Error serialize response");
                        let frame: Vec<u8> = utils::create_websocket_frame(&response_json);
                        stream.write_all(&frame).await.expect("err write response")
                    }
                    Err(_) => {
                        let response = Response {
                            status: String::from("error"),
                            message: chrono::Utc::now().to_string(),
                        };
                        let response_json =
                            ser_to_str(&response).expect("Error serialize response");

                        let frame: Vec<u8> = utils::create_websocket_frame(&response_json);
                        stream.write_all(&frame).await.expect("err write response")
                    }
                };
            } else {
                info!("WebSocket connection closing...");
                break;
            }
        }
    }
}
