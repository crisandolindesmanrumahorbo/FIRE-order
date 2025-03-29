use dotenvy::dotenv;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub fn verify_jwt(token: &str) -> Result<String, &'static str> {
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

fn get_public_key() -> DecodingKey {
    dotenv().ok();
    let key = env::var("JWT_PUBLIC_KEY").expect("Missing JWT_PUBLIC_KEY");
    DecodingKey::from_rsa_pem(key.replace("\\n", "\n").as_bytes()).expect("Invalid public key")
}
