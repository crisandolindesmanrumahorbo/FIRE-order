use std::error::Error;
use stockbit_order_ws::server::Server;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    // Enable logging
    tracing_subscriber::fmt::init();

    Server::start().await;
    Ok(())
}
