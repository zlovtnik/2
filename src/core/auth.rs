use serde::{Deserialize, Serialize};
use argon2::Argon2;
use argon2::password_hash::{SaltString, PasswordHasher, PasswordHash, PasswordVerifier};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, Algorithm, TokenData};
use std::env;
use rand_core::OsRng;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub full_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: uuid::Uuid,
    pub email: String,
    pub full_name: String,
    pub preferences: Option<UserPreferences>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPreferences {
    pub theme: Option<String>,
    pub notifications: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!(e))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = PasswordHash::new(hash);
    if let Ok(parsed_hash) = parsed_hash {
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
    } else {
        false
    }
}

pub fn create_jwt(user_id: uuid::Uuid) -> anyhow::Result<String> {
    let secret = env::var("APP_AUTH__JWT_SECRET").unwrap_or_else(|_| "mysecretkeymysecretkeymysecretkey12".to_string());
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
    };
    let token = encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret.as_bytes()))?;
    Ok(token)
}

pub fn verify_jwt(token: &str) -> anyhow::Result<uuid::Uuid> {
    let secret = env::var("APP_AUTH__JWT_SECRET").unwrap_or_else(|_| "mysecretkeymysecretkeymysecretkey12".to_string());
    let token_data: TokenData<Claims> = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )?;
    let user_id = uuid::Uuid::parse_str(&token_data.claims.sub)?;
    Ok(user_id)
}

pub fn use_verify_jwt_for_warning(token: &str) -> bool {
    verify_jwt(token).is_ok()
} 