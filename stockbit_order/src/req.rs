use anyhow::{Context, Result};
use std::collections::HashMap;
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Debug)]
pub enum Method {
    GET,
    POST,
}

impl TryFrom<&str> for Method {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, anyhow::Error> {
        match value {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            _ => Err(anyhow::anyhow!("Method not supported")),
        }
    }
}

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub params: Option<std::collections::HashMap<String, String>>,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
}

impl Request {
    pub async fn new<Reader: AsyncRead + Unpin>(mut reader: Reader) -> Result<Self> {
        let mut buffer = [0; 1024];
        let size = reader
            .read(&mut buffer)
            .await
            .context("Failed to read stream")?;
        if size >= 1024 {
            return Err(anyhow::anyhow!("Request too large"));
        }
        let request = String::from_utf8_lossy(&buffer[..size]);
        let mut parts = request.split("\r\n\r\n");
        let head = parts.next().context("Headline Error")?;
        // Body
        let body = parts.next().map_or(None, |b| Some(b.to_string()));

        // Method and path
        let mut head_line = head.lines();
        let first: &str = head_line.next().context("Empty Request")?;
        let mut request_parts: std::str::SplitWhitespace<'_> = first.split_whitespace();
        let method: Method = request_parts
            .next()
            .ok_or(anyhow::anyhow!("missing method"))
            .and_then(TryInto::try_into)
            .context("Missing Method")?;
        let url = request_parts.next().context("No Path")?;
        let (path, params) = Self::extract_query_param(&url);

        // Headers
        let mut headers = HashMap::new();
        for line in head_line {
            if let Some((k, v)) = line.split_once(":") {
                headers.insert(k.trim().to_lowercase(), v.trim().to_string());
            }
        }
        Ok(Request {
            method,
            path,
            headers,
            body,
            params,
        })
    }

    fn extract_query_param(url: &str) -> (String, Option<HashMap<String, String>>) {
        // Find the query string
        if let Some(pos) = url.find('?') {
            let path = &url[0..pos];
            let query_string = &url[pos + 1..]; // Get substring after '?'

            // Parse query params into a HashMap
            let params: HashMap<_, _> = query_string
                .split('&')
                .filter_map(|pair| {
                    let mut kv = pair.split('=');
                    Some((kv.next()?.to_string(), kv.next()?.to_string()))
                })
                .collect();

            // Return the token if it exists
            (path.to_string(), Some(params))
            // params.get("token").map(|s| s.to_string())
        } else {
            (url.to_string(), None)
        }
    }
}
