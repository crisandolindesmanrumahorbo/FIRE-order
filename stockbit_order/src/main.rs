use sqlx::Postgres;
use std::error::Error;
use stockbit_order_ws::{cfg::CONFIG, db::Database, redis::RedisCache, server::Server};
use tokio::{
    signal::unix::{SignalKind, signal},
    sync::oneshot::{self, Sender},
    task::JoinHandle,
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    // Enable logging
    tracing_subscriber::fmt::init();

    // Init DB
    let db_pool = Database::new_pool(&CONFIG.database_url).await;
    let redis_conn = RedisCache::new(&CONFIG.redis_url).await?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let server = Server::new(db_pool.clone(), redis_conn.clone());

    let server_handle = tokio::spawn(async move {
        let _ = server.start(shutdown_rx).await;
    });

    gracefully_shutdown(shutdown_tx, server_handle, db_pool).await;
    Ok(())
}

async fn gracefully_shutdown(
    shutdown_tx: Sender<()>,
    server_handle: JoinHandle<()>,
    pool: sqlx::Pool<Postgres>,
) {
    // Wait for shutdown signal
    let mut signal_terminate = signal(SignalKind::terminate()).unwrap();
    let mut signal_interrupt = signal(SignalKind::interrupt()).unwrap();
    tokio::select! {
        _ = signal_terminate.recv() => {
            println!("Shutdown signal received");
        },
        _ = signal_interrupt.recv() => {
            println!("SIGINT received");
        }
    }

    // Trigger graceful shutdown
    let _ = shutdown_tx.send(());
    let _ = server_handle.await;

    // Close DB pool
    pool.close().await;

    println!("Shutdown completed");
}
