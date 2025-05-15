use crate::constant::{LOGGING_HANDSHAKE, LOGGING_MESSAGE};
use crate::logging::thread_logging;
use crate::svc::Service;
use crate::utils;
use anyhow::Result;
use request_http_parser::parser::Request;
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
    user_id: i32,
    svc: &Arc<Service>,
    stream: &mut TcpStream,
) -> Result<()> {
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
    stream
        .write_all(response.as_bytes())
        .await
        .expect("error send handshake success");
    stream.flush().await.expect("error flush");

    thread_logging(LOGGING_HANDSHAKE);
    // Start handling WebSocket messages
    handle_message(stream, user_id, svc).await;

    info!("Closing connection...");
    let _ = stream.shutdown().await;
    Ok(())
}

async fn handle_message(stream: &mut TcpStream, user_id: i32, svc: &Arc<Service>) {
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
                svc.create_order(&message, stream, user_id).await;
            } else {
                info!("WebSocket connection closing...");
                break;
            }
        }
    }
}
