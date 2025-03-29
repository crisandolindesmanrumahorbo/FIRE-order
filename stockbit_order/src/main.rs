use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use stockbit_order_ws::constant::NOT_FOUND;
use stockbit_order_ws::socket::handle_websocket;
use thread_pool::thread_pool::ThreadPool;

fn main() {
    // handle tcp connection
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    println!("Server running on http://127.0.0.1:7878");

    let pool = ThreadPool::new(3);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                pool.execute(move || handle_client(stream));
            }
            Err(err) => println!("unable to connect: {}", err),
        }
    }

    println!("Shutting down.");
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            match &*request {
                r if r.contains("Upgrade: websocket") => handle_websocket(r, &mut stream),
                _ => {
                    stream
                        .write_all(
                            format!("{}{}", NOT_FOUND.to_string(), "404 Not Found".to_string())
                                .as_bytes(),
                        )
                        .unwrap();
                }
            };
        }
        Err(e) => eprintln!("Unable to read stream: {}", e),
    }
}
