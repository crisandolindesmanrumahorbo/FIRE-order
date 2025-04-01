use std::error::Error;
use stockbit_order_ws::constant::{LOGGING_INCOMING_REQUEST, NOT_FOUND};
use stockbit_order_ws::logging::thread_logging;
use stockbit_order_ws::socket::handle_websocket;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init(); // Enable logging

    // handle tcp connection
    let listener = TcpListener::bind("127.0.0.1:7878").await?;
    println!("Server running on http://127.0.0.1:7878");

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            thread_logging(LOGGING_INCOMING_REQUEST);
            if let Err(e) = handle_client(stream).await {
                eprintln!("Connection error: {}", e);
            }
        });
    }
}

async fn handle_client(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    match &*request {
        r if r.contains("Upgrade: websocket") => handle_websocket(r, &mut stream).await.unwrap(),
        _ => {
            stream
                .write_all(
                    format!("{}{}", NOT_FOUND.to_string(), "404 Not Found".to_string()).as_bytes(),
                )
                .await
                .unwrap();
        }
    };
    Ok(())
}
