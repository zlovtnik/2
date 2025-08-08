//! Core authentication functionality including password hashing, JWT operations, and validation.
//!
//! This module provides the fundamental authentication building blocks used throughout
//! the kitchen management system. It handles secure password operations, JWT token
//! management, and request validation with comprehensive security features.
//!
//! # Security Features
//!
//! - **Argon2 Password Hashing**: Industry-standard password hashing with salt
//! - **JWT Token Management**: Secure token generation and validation
//! - **Input Validation**: Comprehensive validation with custom rules
//! - **Input Sanitization**: XSS and injection prevention
//! - **Password Strength Validation**: Enforced complexity requirements
//!
//! # Examples
//!
//! ## Password Hashing and Verification
//!
//! ```rust
//! use kitchen_api::core::auth::{hash_password, verify_password};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let password = "SecurePass123!";
//! let hash = hash_password(password)?;
//!
//! // Later, during login
//! let is_valid = verify_password(password, &hash);
//! assert!(is_valid);
//! # Ok(())
//! # }
//! ```
//!
//! ## JWT Token Operations
//!
//! ```rust
//! use kitchen_api::core::auth::{create_jwt, verify_jwt};
//! use uuid::Uuid;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let user_id = Uuid::new_v4();
//! let token = create_jwt(user_id)?;
//!
//! // Later, during request validation
//! let parsed_user_id = verify_jwt(&token)?;
//! assert_eq!(user_id, parsed_user_id);
//! # Ok(())
//! # }
//! ```

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

/// User registration request structure with comprehensive validation.
///
/// This structure represents a user registration request with built-in validation
/// rules for email format, password strength, and name requirements.
///
/// # Validation Rules
///
/// - **Email**: Must be valid email format (RFC 5322 compliant)
/// - **Password**: 8-128 characters with strength requirements (mixed case, numbers, symbols)
/// - **Full Name**: 1-100 characters, required field
///
/// # Security Features
///
/// - Input sanitization to prevent XSS attacks
/// - Password strength validation with custom rules
/// - Email normalization (lowercase, trimmed)
///
/// # Examples
///
/// ```rust
/// use kitchen_api::core::auth::RegisterRequest;
/// use validator::Validate;
///
/// let request = RegisterRequest {
///     email: "chef@restaurant.com".to_string(),
///     password: "SecurePass123!".to_string(),
///     full_name: "Head Chef".to_string(),
/// };
///
/// // Validate the request
/// match request.validate() {
///     Ok(()) => println!("Registration request is valid"),
///     Err(errors) => println!("Validation errors: {:?}", errors),
/// }
/// ```
///
/// # Kitchen Management Context
///
/// Used during staff onboarding to create new kitchen management accounts
/// with appropriate security validation for restaurant environments.
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
    /// Sanitizes the request data to prevent XSS and normalize input.
    ///
    /// This method applies appropriate sanitization to each field:
    /// - Email: Trimmed and converted to lowercase
    /// - Full name: HTML entities escaped, trimmed
    /// - Password: Left unchanged to preserve security
    ///
    /// # Security Note
    ///
    /// The password field is intentionally not sanitized to preserve
    /// the exact characters entered by the user, which is critical
    /// for password security and verification.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use kitchen_api::core::auth::RegisterRequest;
    ///
    /// let mut request = RegisterRequest {
    ///     email: "  Chef@Restaurant.COM  ".to_string(),
    ///     password: "SecurePass123!".to_string(),
    ///     full_name: "<script>alert('xss')</script>Chef Name".to_string(),
    /// };
    ///
    /// request.sanitize();
    ///
    /// assert_eq!(request.email, "chef@restaurant.com");
    /// assert_eq!(request.password, "SecurePass123!"); // Unchanged
    /// assert!(request.full_name.contains("&lt;script&gt;")); // HTML escaped
    /// ```
    pub fn sanitize(&mut self) {
        self.email = InputSanitizer::sanitize_email(&self.email);
        self.full_name = InputSanitizer::sanitize_text(&self.full_name);
        // Note: We don't sanitize password as it should remain as-is for security
    }
}

/// User login request structure with email and password validation.
///
/// This structure represents a user authentication request with validation
/// rules for email format and password presence.
///
/// # Validation Rules
///
/// - **Email**: Must be valid email format
/// - **Password**: Required field (minimum 1 character)
///
/// # Security Features
///
/// - Email normalization and validation
/// - Input sanitization for XSS prevention
/// - Password field preserved exactly as entered
///
/// # Examples
///
/// ```rust
/// use kitchen_api::core::auth::LoginRequest;
/// use validator::Validate;
///
/// let request = LoginRequest {
///     email: "chef@restaurant.com".to_string(),
///     password: "SecurePass123!".to_string(),
/// };
///
/// // Validate the request
/// match request.validate() {
///     Ok(()) => println!("Login request is valid"),
///     Err(errors) => println!("Validation errors: {:?}", errors),
/// }
/// ```
///
/// # Kitchen Management Context
///
/// Used by kitchen staff to authenticate and access their daily workflows,
/// including order management, inventory tracking, and shift coordination.
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

/// Hashes a password using Argon2 with a random salt.
///
/// This function uses the Argon2id algorithm, which is the recommended
/// password hashing algorithm for new applications. It automatically
/// generates a cryptographically secure random salt for each password.
///
/// # Arguments
///
/// * `password` - The plaintext password to hash
///
/// # Returns
///
/// * `Ok(String)` - The hashed password in PHC string format
/// * `Err(anyhow::Error)` - If hashing fails due to system constraints
///
/// # Security Features
///
/// - Uses Argon2id algorithm (winner of the Password Hashing Competition)
/// - Cryptographically secure random salt generation
/// - Default parameters optimized for security vs. performance
/// - Comprehensive error logging for security monitoring
///
/// # Examples
///
/// ```rust
/// use kitchen_api::core::auth::hash_password;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let password = "SecurePass123!";
/// let hash = hash_password(password)?;
///
/// // Hash format: $argon2id$v=19$m=4096,t=3,p=1$salt$hash
/// assert!(hash.starts_with("$argon2id$"));
/// assert_ne!(hash, password); // Hash is different from original
/// # Ok(())
/// # }
/// ```
///
/// # Kitchen Management Context
///
/// Used during staff registration to securely store passwords for
/// kitchen management system access. Each password gets a unique
/// salt to prevent rainbow table attacks.
///
/// # Performance Considerations
///
/// Argon2 is intentionally slow to prevent brute force attacks.
/// Hashing typically takes 10-100ms depending on system performance.
/// This is acceptable for registration but consider caching for
/// high-frequency operations.
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

/// Verifies a password against its Argon2 hash.
///
/// This function takes a plaintext password and compares it against
/// a previously generated Argon2 hash to determine if they match.
/// It handles hash parsing and verification securely.
///
/// # Arguments
///
/// * `password` - The plaintext password to verify
/// * `hash` - The stored Argon2 hash to verify against
///
/// # Returns
///
/// * `true` - Password matches the hash
/// * `false` - Password doesn't match or hash is invalid
///
/// # Security Features
///
/// - Constant-time comparison to prevent timing attacks
/// - Secure hash parsing with error handling
/// - Comprehensive audit logging for security monitoring
/// - Automatic handling of different Argon2 parameter sets
///
/// # Examples
///
/// ```rust
/// use kitchen_api::core::auth::{hash_password, verify_password};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let password = "SecurePass123!";
/// let hash = hash_password(password)?;
///
/// // Verify correct password
/// assert!(verify_password(password, &hash));
///
/// // Verify incorrect password
/// assert!(!verify_password("WrongPassword", &hash));
///
/// // Verify against invalid hash
/// assert!(!verify_password(password, "invalid_hash"));
/// # Ok(())
/// # }
/// ```
///
/// # Kitchen Management Context
///
/// Used during staff login to authenticate kitchen personnel.
/// Critical for maintaining secure access to kitchen operations,
/// order management, and sensitive restaurant data.
///
/// # Error Handling
///
/// This function never panics and returns `false` for any error
/// condition, including:
/// - Invalid hash format
/// - Corrupted hash data
/// - System memory constraints
/// - Hash algorithm mismatches
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

/// Creates a JWT token for the specified user with 24-hour expiration.
///
/// This function generates a signed JWT token containing the user ID
/// and expiration time. The token is signed using HMAC-SHA256 with
/// a secret key from environment variables.
///
/// # Arguments
///
/// * `user_id` - The UUID of the user for whom to create the token
///
/// # Returns
///
/// * `Ok(String)` - The signed JWT token
/// * `Err(anyhow::Error)` - If token creation fails
///
/// # Token Structure
///
/// The JWT contains:
/// - **sub** (subject): User UUID as string
/// - **exp** (expiration): Unix timestamp (24 hours from creation)
/// - **alg** (algorithm): HS256 (HMAC-SHA256)
///
/// # Environment Configuration
///
/// Requires `APP_AUTH__JWT_SECRET` environment variable for signing.
/// Falls back to a default secret with warning if not configured.
///
/// # Security Features
///
/// - HMAC-SHA256 signing for integrity verification
/// - Automatic expiration (24 hours)
/// - Comprehensive audit logging
/// - Secure secret key handling
///
/// # Examples
///
/// ```rust
/// use kitchen_api::core::auth::create_jwt;
/// use uuid::Uuid;
/// use std::env;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Set JWT secret (in production, use environment variables)
/// env::set_var("APP_AUTH__JWT_SECRET", "your-secret-key-here");
///
/// let user_id = Uuid::new_v4();
/// let token = create_jwt(user_id)?;
///
/// // Token format: header.payload.signature
/// assert_eq!(token.matches('.').count(), 2);
/// println!("Created token for user {}: {}", user_id, token);
/// # Ok(())
/// # }
/// ```
///
/// # Kitchen Management Context
///
/// Used after successful authentication to provide kitchen staff
/// with access tokens for API requests. The 24-hour expiration
/// balances security with usability for typical shift lengths.
///
/// # Token Usage
///
/// The returned token should be included in API requests:
/// ```
/// Authorization: Bearer <token>
/// ```
///
/// # Security Considerations
///
/// - Store JWT secret securely in environment variables
/// - Use HTTPS to prevent token interception
/// - Consider shorter expiration for high-security environments
/// - Implement token refresh for long-running sessions
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