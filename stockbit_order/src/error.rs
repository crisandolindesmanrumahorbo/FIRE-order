use std::{error::Error, fmt::Debug};

#[derive(thiserror::Error)]
pub enum OrderError {
    #[error("Serde error")]
    Serde,

    #[error("Redis error")]
    Redis,

    #[error("Query error")]
    Database,

    #[error("Request body error")]
    BadRequest,
}

impl Debug for OrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)?;
        if let Some(source) = self.source() {
            write!(f, " (Caused by: {})", source)?;
        }
        Ok(())
    }
}
