use anyhow::Result;
use redis::{AsyncCommands, aio::ConnectionManager};
use serde::{Serialize, de::DeserializeOwned};

#[derive(Clone)]
pub struct RedisCache {
    conn: ConnectionManager,
}

impl RedisCache {
    pub async fn new(url: &str) -> Result<Self> {
        let client = redis::Client::open(url).expect("error init redis");
        let conn = ConnectionManager::new(client)
            .await
            .expect("error redis manager");
        Ok(Self { conn })
    }
    pub async fn get_cached<T: DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Result<Option<T>, redis::RedisError> {
        let data: Option<String> = self.conn.get(key).await?;
        match data {
            Some(json) => Ok(serde_json::from_str(&json).ok()),
            None => Ok(None),
        }
    }

    pub async fn set_cache<T: Serialize>(
        &mut self,
        key: &str,
        value: &T,
    ) -> Result<(), redis::RedisError> {
        let json = serde_json::to_string(value).expect("");
        let _: () = self.conn.set(key, json).await.expect("error set");
        Ok(())
    }
}
