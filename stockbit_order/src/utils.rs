use std::collections::HashMap;

use base64::Engine;
use base64::engine::general_purpose;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

pub fn des_from_str<T: for<'a> Deserialize<'a> + Serialize>(
    string: &str,
) -> Result<T, serde_json::Error> {
    serde_json::from_str(string)
}

pub fn ser_to_str<T: for<'a> Deserialize<'a> + Serialize>(
    t: &T,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(t)
}

pub fn extract_token(
    headers: &std::collections::HashMap<std::string::String, std::string::String>,
) -> Option<String> {
    headers.get("authorization").and_then(|s| {
        let mut parts = s.split_whitespace();
        match (parts.next(), parts.next()) {
            (Some("Bearer"), Some(token)) => Some(token.to_string()),
            _ => None,
        }
    })
}

// Generate "Sec-WebSocket-Accept" key using SHA-1 + Base64
pub fn generate_accept_key(key: &str) -> String {
    let magic_string = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let combined = format!("{}{}", key, magic_string);

    let mut hasher = Sha1::new();
    hasher.update(combined.as_bytes());
    let result = hasher.finalize();

    let base64 = general_purpose::STANDARD.encode(&result);
    base64
}

// Parse WebSocket frame and unmask the message
// when sokcet closed on browser, its send closed frame
// already handled in this function
pub fn parse_websocket_framev2(buffer: &[u8]) -> Option<String> {
    if buffer.len() < 2 {
        return None; // Not enough data for a valid frame
    }

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

// Create WebSocket frame to send messages
pub fn create_websocket_frame(message: &str) -> Vec<u8> {
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

pub fn extract_query_param(url: &str) -> Option<HashMap<&str, &str>> {
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
