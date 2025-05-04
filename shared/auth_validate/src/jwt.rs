use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub fn verify_jwt(token: &str, public_key: &str) -> Result<String, &'static str> {
    let dec_key = DecodingKey::from_rsa_pem(public_key.replace("\\n", "\n").as_bytes())
        .expect("Invalid public key");
    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = true; // Ensure expiration is checked
    validation.validate_aud = false; // Disable audience check (optional)

    let token_data = decode::<Claims>(token, &dec_key, &validation).map_err(|e| {
        println!("JWT error: {:?}", e); // Debugging
        "Invalid token"
    })?;

    Ok(token_data.claims.sub)
}
