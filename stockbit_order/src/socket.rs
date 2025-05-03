use crate::constant::{LOGGING_HANDSHAKE, LOGGING_MESSAGE, UNAUTHORIZED};
use crate::logging::thread_logging;
use crate::order::model::{Order, OrderForm};
use crate::utils;
use auth_validate::jwt::verify_jwt;
use request_http_parser::parser::Request;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::info;

pub async fn handle_websocket(
    request: Request,
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
        handle_message(stream, user_id).await;

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
    match verify_jwt(&token) {
        Ok(user_id) => Some(user_id),
        Err(err) => {
            info!("Verification failed: {}", err);
            None
        }
    }

    // request
    //     .params
    //     .as_ref()
    //     .and_then(|params| params.get("token"))
    //     .map(|token| token.to_string())
    //     .and_then(|token| {
    //         verify_jwt(&token)
    //             .map(|_| {
    //                 let sec_websocket_key =
    //                     request.headers.get("sec-websocket-key").map_or("", |v| v);
    //                 let sec_websocket_accept = utils::generate_accept_key(sec_websocket_key);
    //
    //                 format!(
    //                     "HTTP/1.1 101 Switching Protocols\r\n\
    //                 Upgrade: websocket\r\n\
    //                 Connection: Upgrade\r\n\
    //                 Sec-WebSocket-Accept: {}\r\n\
    //                 \r\n",
    //                     sec_websocket_accept
    //                 )
    //             })
    //             .map_err(|err| {
    //                 info!("Verification failed: {}", err);
    //                 err
    //             })
    //             .ok()
    //     })
}

// WebSocket message handling loop (Echo messages)
async fn handle_message(stream: &mut TcpStream, user_id: String) {
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
                        /* TODO
                         * get product_id
                         * */
                        let product_id = 1;
                        let product_name = String::from("");
                        let order = Order::new(order_form, &user_id, product_id, product_name)
                            .expect("parse order");
                        // TODO save order
                        info!("{:?}", order);
                        let response = format!("Echo: {}", order.price);
                        let frame: Vec<u8> = utils::create_websocket_frame(&response);
                        stream.write_all(&frame).await.expect("err write response")
                    }
                    Err(_) => {
                        let frame: Vec<u8> = utils::create_websocket_frame("Order failed");
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
