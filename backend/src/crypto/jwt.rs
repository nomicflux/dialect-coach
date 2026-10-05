use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

fn get_secret() -> Result<String, jsonwebtoken::errors::Error> {
    std::env::var("JWT_SECRET")
        .map_err(|_| jsonwebtoken::errors::ErrorKind::InvalidKeyFormat.into())
}

fn expiration_time() -> usize {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    (now + 24 * 60 * 60) as usize
}

pub fn generate_token(user_id: Uuid) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration_time(),
    };
    let secret = get_secret()?;
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Whether `validate_token` rejects an expired token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expiry {
    Enforce,
    Ignore,
}

fn validation(expiry: Expiry) -> Validation {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = expiry == Expiry::Enforce;
    validation
}

pub fn validate_token(token: &str, expiry: Expiry) -> Result<Uuid, jsonwebtoken::errors::Error> {
    let secret = get_secret()?;
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation(expiry),
    )?;
    Uuid::parse_str(&token_data.claims.sub)
        .map_err(|_| jsonwebtoken::errors::ErrorKind::InvalidSubject.into())
}
