use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

use crate::req::{Method::GET, Request};
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
        let (reader, _) = stream.split();
        let request = match Request::new(reader).await {
            Ok(req) => req,
            Err(err) => {
                return stream
                    .write_all(format!("{}{}", crate::constant::BAD_REQUEST, err).as_bytes())
                    .await
                    .context("error write");
            }
        };
        println!("{:#?}", request);

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
