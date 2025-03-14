use base64::Engine;
use base64::engine::general_purpose;
use dotenvy::dotenv;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::env;
use std::net::{TcpListener, TcpStream};
use std::{
    collections::HashMap,
    io::{Read, Write},
};
use stockbit_order_ws::ThreadPool;

#[derive(Serialize, Deserialize, Debug)]
struct User {
    pub id: Option<i32>,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const UNATHORIZED: &str = "HTTP/1.1 401 Unathorized\r\n\r\n";
const INTERNAL_ERROR: &str = "HTTP/1.1 500 INTERNAL ERROR\r\n\r\n";

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

            if request.contains("Upgrade: websocket") {
                if let Some(response) = handle_websocket(&request) {
                    stream.write_all(response.as_bytes()).unwrap();
                    stream.flush().unwrap();
                    println!("WebSocket handshake successful!");
                    // Start handling WebSocket messages
                    websocket_loop(&mut stream);

                    println!("Closing connection...");
                    stream.shutdown(std::net::Shutdown::Both).ok();
                } else {
                    println!("WebSocket handshake failed!");
                    stream
                        .write_all(
                            format!(
                                "{}{}",
                                UNATHORIZED.to_string(),
                                "401 unathorized".to_string()
                            )
                            .as_bytes(),
                        )
                        .unwrap();
                }
            }
        }
        Err(e) => eprintln!("Unable to read stream: {}", e),
    }
}

fn handle_websocket(request: &str) -> Option<String> {
    println!("Client is requesting WebSocket connection");

    if let Some(params) = extract_query_param(request) {
        if let Some(token) = params.get("token").map(|s| s.to_string()) {
            match verify_jwt(&token) {
                Ok(_) => {
                    let sec_websocket_key = extract_key(&request);
                    let sec_websocket_accept = generate_accept_key(&sec_websocket_key);

                    // Send WebSocket handshake response
                    let response = format!(
                        "HTTP/1.1 101 Switching Protocols\r\n\
                    Upgrade: websocket\r\n\
                    Connection: Upgrade\r\n\
                    Sec-WebSocket-Accept: {}\r\n\
                    \r\n",
                        sec_websocket_accept
                    );
                    Some(response)
                }
                Err(err) => {
                    println!("Verification failed: {}", err);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn verify_jwt(token: &str) -> Result<String, &'static str> {
    let public_key = get_public_key();

    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = true; // Ensure expiration is checked
    validation.validate_aud = false; // Disable audience check (optional)

    let token_data = decode::<Claims>(token, &public_key, &validation).map_err(|e| {
        println!("JWT error: {:?}", e); // Debugging
        "Invalid token"
    })?;

    Ok(token_data.claims.sub)
}

// Extract "Sec-WebSocket-Key" from client request
fn extract_key(request: &str) -> String {
    for line in request.lines() {
        if line.starts_with("Sec-WebSocket-Key:") {
            return line.split(": ").nth(1).unwrap().trim().to_string();
        }
    }
    String::new()
}

// Generate "Sec-WebSocket-Accept" key using SHA-1 + Base64
fn generate_accept_key(key: &str) -> String {
    let magic_string = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let combined = format!("{}{}", key, magic_string);

    let mut hasher = Sha1::new();
    hasher.update(combined.as_bytes());
    let result = hasher.finalize();

    let base64 = general_purpose::STANDARD.encode(&result);
    base64
}

// WebSocket message handling loop (Echo messages)
fn websocket_loop(stream: &mut TcpStream) {
    let mut buffer = [0; 1024];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("Client disconnected");
                break;
            }
            Ok(size) => {
                if let Some(message) = parse_websocket_framev2(&buffer[..size]) {
                    println!("Received WebSocket message: {}", message);

                    let response = format!("Echo: {}", message);
                    let frame = create_websocket_frame(&response);
                    stream.write_all(&frame).unwrap();
                } else {
                    println!("WebSocket connection closing...");
                    break;
                }
            }
            Err(_) => {
                println!("Connection error");
                break;
            }
        }
    }
}

fn parse_websocket_framev2(buffer: &[u8]) -> Option<String> {
    if buffer.len() < 2 {
        return None; // Not enough data for a valid frame
    }

    let fin = (buffer[0] & 0b10000000) != 0; // FIN bit
    let opcode = buffer[0] & 0b00001111; // Extract opcode

    let masked = (buffer[1] & 0b10000000) != 0; // Mask bit
    let mut payload_length = (buffer[1] & 0b01111111) as usize;

    let mut index = 2;

    // Extended payload lengths
    if payload_length == 126 {
        if buffer.len() < 4 {
            return None; // Not enough data
        }
        payload_length = u16::from_be_bytes([buffer[2], buffer[3]]) as usize;
        index += 2;
    } else if payload_length == 127 {
        if buffer.len() < 10 {
            return None; // Not enough data
        }
        payload_length = u64::from_be_bytes([
            buffer[2], buffer[3], buffer[4], buffer[5], buffer[6], buffer[7], buffer[8], buffer[9],
        ]) as usize;
        index += 8;
    }

    // Ensure enough data is available
    if buffer.len() < index + (if masked { 4 } else { 0 }) + payload_length {
        return None;
    }

    let masking_key = if masked {
        Some(&buffer[index..index + 4])
    } else {
        None
    };
    index += if masked { 4 } else { 0 };

    let mut decoded_payload = Vec::new();
    for (i, &byte) in buffer[index..index + payload_length].iter().enumerate() {
        decoded_payload.push(if let Some(key) = masking_key {
            byte ^ key[i % 4] // Unmask the message
        } else {
            byte
        });
    }

    match opcode {
        0x1 => {
            Some(String::from_utf8(decoded_payload).unwrap_or_else(|_| "Invalid UTF-8".to_string()))
        } // Text frame
        0x2 => Some("<Binary Frame>".to_string()), // Binary frame (not handled here)
        0x8 => {
            println!("Received Close Frame!");
            None // Close frame, no need to return message
        }
        0x9 => {
            println!("Received Ping Frame!");
            None // Ping frame
        }
        0xA => {
            println!("Received Pong Frame!");
            None // Pong frame
        }
        _ => {
            println!("Unknown WebSocket Frame: Opcode {}", opcode);
            None
        }
    }
}

// Parse WebSocket frame and unmask the message
fn parse_websocket_frame(buffer: &[u8]) -> String {
    if buffer.len() < 6 {
        return String::new(); // Invalid frame
    }

    let payload_length = buffer[1] & 127;
    let masking_key = &buffer[2..6];
    let mut decoded = Vec::new();

    for (i, &byte) in buffer[6..(6 + payload_length as usize)].iter().enumerate() {
        decoded.push(byte ^ masking_key[i % 4]); // Unmask the message
    }

    String::from_utf8(decoded).unwrap_or_else(|_| "Invalid UTF-8".to_string())
}

// Create WebSocket frame to send messages
fn create_websocket_frame(message: &str) -> Vec<u8> {
    let mut frame = vec![0x81]; // FIN + Text frame opcode
    let payload = message.as_bytes();

    if payload.len() <= 125 {
        frame.push(payload.len() as u8);
    } else {
        frame.push(126);
        frame.push(((payload.len() >> 8) & 255) as u8);
        frame.push((payload.len() & 255) as u8);
    }

    frame.extend_from_slice(payload);
    frame
}

fn get_public_key() -> DecodingKey {
    dotenv().ok();
    let key = env::var("JWT_PUBLIC_KEY").expect("Missing JWT_PUBLIC_KEY");
    DecodingKey::from_rsa_pem(key.replace("\\n", "\n").as_bytes()).expect("Invalid public key")
}

fn extract_query_param(request: &str) -> Option<HashMap<&str, &str>> {
    // Get the first line of the request
    let first_line = request.lines().next()?;

    // Split by space to get the path
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let url = parts[1]; // The URL path with query string

    // Find the query string
    if let Some(pos) = url.find('?') {
        let query_string = &url[pos + 1..]; // Get substring after '?'

        // Parse query params into a HashMap
        let params: HashMap<_, _> = query_string
            .split('&')
            .filter_map(|pair| {
                let mut kv = pair.split('=');
                Some((kv.next()?, kv.next()?))
            })
            .collect();

        // Return the token if it exists
        Some(params)
        // params.get("token").map(|s| s.to_string())
    } else {
        None
    }
}
