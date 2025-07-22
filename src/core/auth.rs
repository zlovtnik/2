use serde::{Deserialize, Serialize};
use argon2::Argon2;
use argon2::password_hash::{SaltString, PasswordHasher, PasswordHash, PasswordVerifier};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, Algorithm, TokenData};
use std::env;
use rand_core::OsRng;
use utoipa::ToSchema;
use tracing::{info, warn, error, debug};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub full_name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    pub id: uuid::Uuid,
    pub email: String,
    pub full_name: String,
    pub preferences: Option<UserPreferences>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
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
    debug!("Starting password hashing operation");
    let salt = SaltString::generate(&mut OsRng);
    debug!("Generated salt for password hashing");
    
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)
        .map_err(|e| {
            error!(error = %e, "Password hashing failed");
            anyhow::anyhow!(e)
        })?
        .to_string();
    
    debug!("Password hashed successfully");
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    debug!("Starting password verification");
    let parsed_hash = PasswordHash::new(hash);
    if let Ok(parsed_hash) = parsed_hash {
        let verification_result = Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok();
        if verification_result {
            debug!("Password verification successful");
        } else {
            warn!("Password verification failed - invalid password");
        }
        verification_result
    } else {
        error!("Failed to parse password hash");
        false
    }
}

pub fn create_jwt(user_id: uuid::Uuid) -> anyhow::Result<String> {
    info!(user_id = %user_id, "Creating JWT token");
    debug!("Loading JWT secret from environment");
    
    let secret = env::var("APP_AUTH__JWT_SECRET").unwrap_or_else(|_| {
        warn!("JWT secret not found in environment, using default");
        "mysecretkeymysecretkeymysecretkey12".to_string()
    });
    
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;
    
    debug!(user_id = %user_id, expiration = expiration, "Creating JWT claims");
    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
    };
    
    debug!("Encoding JWT token");
    let token = encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| {
            error!(user_id = %user_id, error = %e, "Failed to encode JWT token");
            anyhow::anyhow!(e)
        })?;
    
    info!(user_id = %user_id, "JWT token created successfully");
    Ok(token)
}

pub fn verify_jwt(token: &str) -> anyhow::Result<uuid::Uuid> {
    debug!("Starting JWT token verification");
    debug!("Loading JWT secret from environment");
    
    let secret = env::var("APP_AUTH__JWT_SECRET").unwrap_or_else(|_| {
        warn!("JWT secret not found in environment, using default");
        "mysecretkeymysecretkeymysecretkey12".to_string()
    });
    
    debug!("Decoding JWT token");
    let token_data: TokenData<Claims> = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    ).map_err(|e| {
        warn!(error = %e, "JWT token verification failed");
        anyhow::anyhow!(e)
    })?;
    
    debug!("Parsing user ID from JWT claims");
    let user_id = uuid::Uuid::parse_str(&token_data.claims.sub)
        .map_err(|e| {
            error!(error = %e, "Failed to parse user ID from JWT claims");
            anyhow::anyhow!(e)
        })?;
    
    info!(user_id = %user_id, "JWT token verified successfully");
    Ok(user_id)
}

pub fn use_verify_jwt_for_warning(token: &str) -> bool {
    verify_jwt(token).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use uuid::Uuid;

    #[test]
    fn test_hash_and_verify_password() {
        let password = "test_password";
        let hash = hash_password(password).expect("Hashing should succeed");
        assert!(verify_password(password, &hash));
        assert!(!verify_password("wrong_password", &hash));
    }

    #[test]
    fn test_create_and_verify_jwt() {
        // Set a fixed secret for deterministic tests
        env::set_var("APP_AUTH__JWT_SECRET", "testsecretkeytestsecretkeytestsecr");
        let user_id = Uuid::new_v4();
        let token = create_jwt(user_id).expect("JWT creation should succeed");
        let parsed_id = verify_jwt(&token).expect("JWT verification should succeed");
        assert_eq!(user_id, parsed_id);
    }

    #[test]
    fn test_verify_jwt_invalid_token() {
        env::set_var("APP_AUTH__JWT_SECRET", "testsecretkeytestsecretkeytestsecr");
        let invalid_token = "invalid.token.value";
        let result = verify_jwt(invalid_token);
        assert!(result.is_err());
    }
} 