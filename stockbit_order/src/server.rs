use anyhow::{Context, Result};
use request_http_parser::parser::{Method::GET, Request};
use sqlx::{Pool, Postgres};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot::Receiver;

use crate::constant::BAD_REQUEST;
use crate::order::repo::OrderRepo;
use crate::product::repo::ProductRepository;
use crate::{constant, socket};
use std::sync::Arc;

pub struct Server {
    product_repo: Arc<ProductRepository>,
    order_repo: Arc<OrderRepo>,
}

impl Server {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self {
            product_repo: Arc::new(ProductRepository::new(pool.clone())),
            order_repo: Arc::new(OrderRepo::new(pool.clone())),
        }
    }
    pub async fn start(self, mut shutdown_rx: Receiver<()>) -> anyhow::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();
        println!("Server running on http://127.0.0.1:7878");

        loop {
            tokio::select! {
                conn = listener.accept() => {
                    let ( stream, _) = conn?;
                    let product_repo = Arc::clone(&self.product_repo);
                    let order_repo = Arc::clone(&self.order_repo);
                    tokio::spawn(async move {
                        crate::logging::thread_logging(crate::constant::LOGGING_INCOMING_REQUEST);
                        if let Err(e) = Self::handle_client(stream, &product_repo, &order_repo).await {
                            eprintln!("Connection error: {}", e);
                        }
                });
                },
                _ = &mut shutdown_rx => {
                    println!("shutting down ...");
                    break;
                }
            }
        }
        Ok(())
    }

    async fn handle_client(
        mut stream: TcpStream,
        product_repo: &Arc<ProductRepository>,
        order_repo: &Arc<OrderRepo>,
    ) -> Result<()> {
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
            (GET, "/order/ws") => {
                socket::handle_websocket(request, product_repo, order_repo, &mut stream)
                    .await
                    .unwrap()
            }
            _ => {
                stream
                    .write_all(format!("{}{}", constant::NOT_FOUND, "404 Not Found").as_bytes())
                    .await?;
            }
        };
        Ok(())
    }
}
