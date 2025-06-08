use anyhow::Result;

use request_http_parser::parser::{Method::GET, Method::POST};
use sqlx::{Pool, Postgres};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot::Receiver;
use tracing::info;

use crate::account::repo::AccountRepo;
use crate::mdw::Middleware;
use crate::order::repo::OrderRepo;
use crate::portfolio::repo::PortoRepo;
use crate::product::repo::ProductRepository;
use crate::redis::RedisCache;
use crate::svc::Service;
use crate::{constant, socket};
use std::sync::Arc;

pub struct Server {
    svc: Arc<Service>,
}

impl Server {
    pub fn new(pool: Pool<Postgres>, redis_cache: RedisCache) -> Self {
        Self {
            svc: Arc::new(Service::new(
                ProductRepository::new(pool.clone()),
                OrderRepo::new(pool.clone()),
                AccountRepo::new(pool.clone()),
                PortoRepo::new(pool.clone()),
                redis_cache,
            )),
        }
    }
    pub async fn start(self, mut shutdown_rx: Receiver<()>) -> anyhow::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();
        println!("Server running on http://127.0.0.1:7878");

        loop {
            tokio::select! {
                conn = listener.accept() => {
                    let ( stream, _) = conn?;
                    let svc = Arc::clone(&self.svc);
                    tokio::spawn(async move {
                        crate::logging::thread_logging(crate::constant::LOGGING_INCOMING_REQUEST);
                        if let Err(e) = Self::handle_client(stream, &svc).await {
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

    async fn handle_client(mut stream: TcpStream, svc: &Arc<Service>) -> Result<()> {
        let (request, user_id) = match Middleware::new(&mut stream).await {
            Ok((request, user_id)) => (request, user_id),
            Err(e) => {
                info!("error {}", e);
                stream
                    .write_all(format!("{}{}", constant::BAD_REQUEST, "").as_bytes())
                    .await?;
                return Ok(());
            }
        };
        let (_, mut writer) = stream.split();

        //Router
        match (&request.method, request.path.as_str()) {
            (GET, "/order/ws") => socket::handle_websocket(request, user_id, svc, &mut stream)
                .await
                .expect("error handle ws"),
            (POST, "/order") => svc
                .create_order_nonws(request, &mut writer)
                .await
                .expect("error create order nonws"),
            (GET, "/order") => svc
                .get_orders(user_id, &mut writer)
                .await
                .expect("error get orders"),
            (GET, "/portfolio") => svc
                .get_portfolios(request, user_id, &mut writer)
                .await
                .expect("error get portfolios"),
            (GET, "/account") => svc
                .get_account(request, user_id, &mut writer)
                .await
                .expect("error get account"),

            _ => {
                stream
                    .write_all(format!("{}{}", constant::NOT_FOUND, "404 Not Found").as_bytes())
                    .await?;
            }
        };
        Ok(())
    }
}
