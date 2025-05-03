use anyhow::{Context, Result};
use request_http_parser::parser::{Method::GET, Request};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::constant::BAD_REQUEST;
use crate::{constant, socket};

pub struct Server {}

impl Server {
    pub async fn start() {
        let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();
        println!("Server running on http://127.0.0.1:7878");

        loop {
            let (stream, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                crate::logging::thread_logging(crate::constant::LOGGING_INCOMING_REQUEST);
                if let Err(e) = Self::handle_client(stream).await {
                    eprintln!("Connection error: {}", e);
                }
            });
        }
    }

    async fn handle_client(mut stream: TcpStream) -> Result<()> {
        let (mut reader, mut writer) = stream.split();

        let mut buffer = [0; 1024];
        let size = reader
            .read(&mut buffer)
            .await
            .context("Failed to read stream")?;
        if size >= 1024 {
            let _ = writer
                .write_all(format!("{}{}", BAD_REQUEST, "Requets too large").as_bytes())
                .await
                .context("Failed to write");

            let _ = writer.flush().await.context("Failed to flush");

            return Ok(());
        }
        let req_str = String::from_utf8_lossy(&buffer[..size]);
        let request = match Request::new(&req_str) {
            Ok(req) => req,
            Err(e) => {
                println!("{}", e);
                let _ = writer
                    .write_all(format!("{}{}", BAD_REQUEST, e).as_bytes())
                    .await
                    .context("Failed to write");

                let _ = writer.flush().await.context("Failed to flush");
                return Ok(());
            }
        };

        //Router
        match (&request.method, request.path.as_str()) {
            (GET, "/order/ws") => socket::handle_websocket(request, &mut stream)
                .await
                .unwrap(),
            _ => {
                stream
                    .write_all(format!("{}{}", constant::NOT_FOUND, "404 Not Found").as_bytes())
                    .await?;
            }
        };
        Ok(())
    }
}
