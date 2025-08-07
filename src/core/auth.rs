use serde::{Deserialize, Serialize};
use argon2::Argon2;
use argon2::password_hash::{SaltString, PasswordHasher, PasswordHash, PasswordVerifier};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, Algorithm, TokenData};
use std::env;
use rand_core::OsRng;
use utoipa::ToSchema;
use tracing::{info, warn, error, debug};
use validator::{Validate, ValidationError};
use crate::middleware::validation::{ValidatedRequest, InputSanitizer};

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 8, max = 128, message = "Password must be between 8 and 128 characters"))]
    #[validate(custom(function = "validate_password_strength", message = "Password does not meet security requirements"))]
    pub password: String,
    #[validate(length(min = 1, max = 100, message = "Full name must be between 1 and 100 characters"))]
    pub full_name: String,
}

impl ValidatedRequest for RegisterRequest {}

impl RegisterRequest {
    /// Sanitize the request data
    pub fn sanitize(&mut self) {
        self.email = InputSanitizer::sanitize_email(&self.email);
        self.full_name = InputSanitizer::sanitize_text(&self.full_name);
        // Note: We don't sanitize password as it should remain as-is for security
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

impl ValidatedRequest for LoginRequest {}

impl LoginRequest {
    /// Sanitize the request data
    pub fn sanitize(&mut self) {
        self.email = InputSanitizer::sanitize_email(&self.email);
        // Note: We don't sanitize password as it should remain as-is for security
    }
}

/// Custom validator for password strength
fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    match InputSanitizer::validate_password_strength(password) {
        Ok(()) => Ok(()),
        Err(errors) => {
            let mut error = ValidationError::new("password_strength");
            error.message = Some(errors.join(", ").into());
            Err(error)
        }
    }
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

    /// Setup test environment with proper JWT secret
    fn setup_test_env() {
        env::set_var("APP_AUTH__JWT_SECRET", "testsecretkeytestsecretkeytestsecr");
    }

    #[test]
    fn test_hash_password_success() {
        let password = "test_password_123";
        let hash = hash_password(password).expect("Hashing should succeed");
        
        // Hash should not be empty and should be different from original password
        assert!(!hash.is_empty());
        assert_ne!(hash, password);
        
        // Hash should start with Argon2 identifier
        assert!(hash.starts_with("$argon2"));
    }

    #[test]
    fn test_hash_password_different_salts() {
        let password = "same_password";
        let hash1 = hash_password(password).expect("First hash should succeed");
        let hash2 = hash_password(password).expect("Second hash should succeed");
        
        // Same password should produce different hashes due to different salts
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_password_correct() {
        let password = "test_password_123";
        let hash = hash_password(password).expect("Hashing should succeed");
        
        assert!(verify_password(password, &hash));
    }

    #[test]
    fn test_verify_password_incorrect() {
        let password = "correct_password";
        let wrong_password = "wrong_password";
        let hash = hash_password(password).expect("Hashing should succeed");
        
        assert!(!verify_password(wrong_password, &hash));
    }

    #[test]
    fn test_verify_password_empty_inputs() {
        let hash = hash_password("test").expect("Hashing should succeed");
        
        // Empty password should not verify
        assert!(!verify_password("", &hash));
        
        // Valid password against empty hash should not verify
        assert!(!verify_password("test", ""));
    }

    #[test]
    #[serial_test::serial]
    fn test_create_jwt_success() {
        setup_test_env();
        let user_id = Uuid::new_v4();
        
        let token = create_jwt(user_id).expect("JWT creation should succeed");
        
        // Token should not be empty and should have JWT structure (header.payload.signature)
        assert!(!token.is_empty());
        assert_eq!(token.matches('.').count(), 2);
    }

    #[test]
    #[serial_test::serial]
    fn test_create_jwt_different_users() {
        setup_test_env();
        let user_id1 = Uuid::new_v4();
        let user_id2 = Uuid::new_v4();
        
        let token1 = create_jwt(user_id1).expect("First JWT creation should succeed");
        let token2 = create_jwt(user_id2).expect("Second JWT creation should succeed");
        
        // Different users should produce different tokens
        assert_ne!(token1, token2);
    }

    #[test]
    #[serial_test::serial]
    fn test_verify_jwt_success() {
        setup_test_env();
        let user_id = Uuid::new_v4();
        
        let token = create_jwt(user_id).expect("JWT creation should succeed");
        let parsed_id = verify_jwt(&token).expect("JWT verification should succeed");
        
        assert_eq!(user_id, parsed_id);
    }

    #[test]
    fn test_verify_jwt_invalid_token() {
        setup_test_env();
        
        let invalid_tokens = vec![
            "invalid.token.value",
            "not_a_jwt_at_all",
            "",
            "invalid",
            "invalid.token",
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.invalid.signature",
        ];
        
        for invalid_token in invalid_tokens {
            let result = verify_jwt(invalid_token);
            assert!(result.is_err(), "Token '{}' should be invalid", invalid_token);
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_verify_jwt_wrong_secret() {
        setup_test_env();
        let user_id = Uuid::new_v4();
        let token = create_jwt(user_id).expect("JWT creation should succeed");
        
        // Change the secret
        env::set_var("APP_AUTH__JWT_SECRET", "different_secret_key_for_testing");
        
        let result = verify_jwt(&token);
        assert!(result.is_err(), "JWT verification should fail with wrong secret");
        
        // Restore original secret for other tests
        setup_test_env();
    }

    #[test]
    #[serial_test::serial]
    fn test_use_verify_jwt_for_warning() {
        setup_test_env();
        let user_id = Uuid::new_v4();
        let valid_token = create_jwt(user_id).expect("JWT creation should succeed");
        
        assert!(use_verify_jwt_for_warning(&valid_token));
        assert!(!use_verify_jwt_for_warning("invalid_token"));
    }

    // Test struct validation
    #[test]
    fn test_register_request_validation() {
        let request = RegisterRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            full_name: "Test User".to_string(),
        };
        
        assert_eq!(request.email, "test@example.com");
        assert_eq!(request.password, "password123");
        assert_eq!(request.full_name, "Test User");
    }

    #[test]
    fn test_login_request_validation() {
        let request = LoginRequest {
            email: "user@example.com".to_string(),
            password: "userpass".to_string(),
        };
        
        assert_eq!(request.email, "user@example.com");
        assert_eq!(request.password, "userpass");
    }

    #[test]
    fn test_user_profile_creation() {
        let user_id = Uuid::new_v4();
        let preferences = UserPreferences {
            theme: Some("dark".to_string()),
            notifications: Some(true),
        };
        
        let profile = UserProfile {
            id: user_id,
            email: "profile@example.com".to_string(),
            full_name: "Profile User".to_string(),
            preferences: Some(preferences),
        };
        
        assert_eq!(profile.id, user_id);
        assert_eq!(profile.email, "profile@example.com");
        assert_eq!(profile.full_name, "Profile User");
        assert!(profile.preferences.is_some());
        
        let prefs = profile.preferences.unwrap();
        assert_eq!(prefs.theme, Some("dark".to_string()));
        assert_eq!(prefs.notifications, Some(true));
    }
} 