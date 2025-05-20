use anyhow::{Context, Result, anyhow};
use auth_validate::jwt::verify_jwt;
use request_http_parser::parser::Request;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::cfg::CONFIG;
use crate::constant::{BAD_REQUEST, UNAUTHORIZED};
use crate::utils::extract_token;

pub struct Middleware {}

impl Middleware {
    pub async fn new(stream: &mut TcpStream) -> Result<(Request, i32)> {
        let mut buffer = [0; 1024];
        let size = stream
            .read(&mut buffer)
            .await
            .context("Failed to read stream")?;
        if size >= 1024 {
            let _ = stream
                .write_all(format!("{}{}", BAD_REQUEST, "Requets too large").as_bytes())
                .await
                .context("Failed to write");

            let _ = stream.flush().await.context("Failed to flush");

            return Err(anyhow!("request too large"));
        }
        let req_str = String::from_utf8_lossy(&buffer[..size]);
        let request = match Request::new(&req_str) {
            Ok(req) => req,
            Err(e) => {
                println!("{}", e);
                let _ = stream
                    .write_all(format!("{}{}", BAD_REQUEST, e).as_bytes())
                    .await
                    .context("Failed to write");

                let _ = stream.flush().await.context("Failed to flush");
                return Err(anyhow!("request format invalid"));
            }
        };
        let token_opt: Option<String>;
        // ws
        if request.path.contains("ws") {
            token_opt = match &request.params {
                Some(params) => match params.get("token") {
                    Some(token) => Some(token.to_string()),
                    _ => None,
                },
                _ => None,
            };
        // non ws
        } else {
            token_opt = match extract_token(&request.headers) {
                Some(token) => Some(token),
                _ => None,
            };
        }
        let token = match token_opt {
            Some(token) => token,
            None => {
                stream
                    .write_all(
                        format!(
                            "{}{}",
                            UNAUTHORIZED.to_string(),
                            "401 unathorized".to_string()
                        )
                        .as_bytes(),
                    )
                    .await?;
                return Err(anyhow!("extract token error"));
            }
        };

        let user_id = match verify_jwt(&token, &CONFIG.jwt_public_key) {
            Ok(user_id) => user_id,
            Err(_) => {
                stream
                    .write_all(
                        format!(
                            "{}{}",
                            UNAUTHORIZED.to_string(),
                            "401 unathorized".to_string()
                        )
                        .as_bytes(),
                    )
                    .await?;
                return Err(anyhow!("token unathorized"));
            }
        };
        Ok((
            request,
            user_id.parse::<i32>().expect("error parsed user_id"),
        ))
    }
}
