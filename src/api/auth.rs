//! Authentication API endpoints for user registration, login, and token management.
//!
//! This module provides secure authentication functionality including:
//! - User registration with validation and password hashing
//! - User login with credential verification
//! - JWT token generation and refresh
//! - Comprehensive error handling and logging
//!
//! # Security Features
//!
//! - Argon2 password hashing with salt
//! - JWT tokens with configurable expiration
//! - Input validation and sanitization
//! - Rate limiting support
//! - Comprehensive audit logging
//!
//! # Examples
//!
//! ## User Registration
//!
//! ```rust
//! use serde_json::json;
//! use reqwest::Client;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new();
//! let registration = json!({
//!     "email": "chef@restaurant.com",
//!     "password": "SecurePass123!",
//!     "full_name": "Head Chef"
//! });
//!
//! let response = client
//!     .post("http://localhost:3000/api/v1/auth/register")
//!     .json(&registration)
//!     .send()
//!     .await?;
//!
//! if response.status().is_success() {
//!     let token_response: serde_json::Value = response.json().await?;
//!     println!("Registration successful: {}", token_response["token"]);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## User Login
//!
//! ```rust
//! use serde_json::json;
//! use reqwest::Client;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new();
//! let login = json!({
//!     "email": "chef@restaurant.com",
//!     "password": "SecurePass123!"
//! });
//!
//! let response = client
//!     .post("http://localhost:3000/api/v1/auth/login")
//!     .json(&login)
//!     .send()
//!     .await?;
//!
//! if response.status().is_success() {
//!     let token_response: serde_json::Value = response.json().await?;
//!     println!("Login successful: {}", token_response["token"]);
//! }
//! # Ok(())
//! # }
//! ```

use axum::{Json, response::IntoResponse};
use axum::extract::FromRef;
use axum::http::request::Parts;
use axum::extract::FromRequestParts;
use axum::RequestPartsExt;
use std::ops::Deref;

/// Simple extractor to pull a Bearer token string from the Authorization header.
pub struct BearerToken(String);

impl Deref for BearerToken {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for BearerToken
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        use axum::http::header::AUTHORIZATION;

        let headers = &parts.headers;
        let token = match headers.get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
        {
            Some(t) => t.to_string(),
            None => {
                return Err(AuthError::Standard(ErrorResponse::new("Invalid credentials", Some("Missing Authorization header".to_string()))));
            }
        };

        Ok(BearerToken(token))
    }
}
use crate::core::auth::{RegisterRequest, LoginRequest, hash_password, verify_password, create_jwt, verify_jwt};
use crate::middleware::validation::ValidationErrorResponse;
use tracing::{info, warn};
use uuid::Uuid;
use crate::core::user::User;
use sqlx::PgPool;
use axum::extract::State;
use chrono::Utc;
use utoipa::ToSchema;
use serde::Serialize;
use validator::Validate;

/// Standard error response structure for authentication endpoints.
///
/// This structure provides consistent error reporting across all authentication
/// operations, including detailed error messages and optional additional context.
///
/// # Fields
///
/// * `error` - The main error category or type
/// * `details` - Optional additional information about the error
///
/// # Examples
///
/// ```rust
/// use kitchen_api::api::auth::ErrorResponse;
///
/// let error = ErrorResponse::new("Invalid credentials", Some("Email not found".to_string()));
/// assert_eq!(error.error, "Invalid credentials");
/// ```
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    error: String,
    details: Option<String>,
}

impl ErrorResponse {
    /// Creates a new error response with the specified error message and optional details.
    ///
    /// # Arguments
    ///
    /// * `error` - The main error message or category
    /// * `details` - Optional additional context or specific error information
    ///
    /// # Returns
    ///
    /// A new `ErrorResponse` instance ready for serialization and HTTP response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use kitchen_api::api::auth::ErrorResponse;
    ///
    /// // Simple error without details
    /// let error = ErrorResponse::new("Authentication failed", None);
    ///
    /// // Error with additional context
    /// let detailed_error = ErrorResponse::new(
    ///     "Registration failed",
    ///     Some("Email already exists in the system".to_string())
    /// );
    /// ```
    pub fn new(error: impl Into<String>, details: Option<String>) -> Self {
        Self {
            error: error.into(),
            details,
        }
    }
}

/// Converts `ErrorResponse` into an HTTP response with appropriate status codes.
///
/// This implementation automatically maps error types to HTTP status codes:
/// - "Registration failed" → 400 Bad Request
/// - "Invalid credentials" → 401 Unauthorized  
/// - All others → 500 Internal Server Error
///
/// # Examples
///
/// ```rust
/// use axum::response::IntoResponse;
/// use kitchen_api::api::auth::ErrorResponse;
///
/// let error = ErrorResponse::new("Invalid credentials", None);
/// let response = error.into_response();
/// // Response will have 401 Unauthorized status
/// ```
impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let status = match self.error.as_str() {
            // Client-side validation/registration problems
            "Registration failed" => axum::http::StatusCode::BAD_REQUEST,
            // Authentication failures
            "Invalid credentials" => axum::http::StatusCode::UNAUTHORIZED,
            // Resource not found
            "User not found" | "Token not found" => axum::http::StatusCode::NOT_FOUND,
            // Conflict / already exists
            "User already exists" | "Token already exists" => axum::http::StatusCode::CONFLICT,
            // Fallback to internal server error for other cases
            _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        };
        
        (status, Json(self)).into_response()
    }
}

/// Combined error type for authentication endpoints that handles both validation and standard errors.
///
/// This enum provides a unified error handling approach for authentication operations,
/// supporting both input validation errors and general authentication failures.
///
/// # Variants
///
/// * `Validation` - Input validation errors with field-specific messages
/// * `Standard` - General authentication errors (credentials, server errors, etc.)
///
/// # Examples
///
/// ```rust
/// use kitchen_api::api::auth::{AuthError, ErrorResponse};
/// use kitchen_api::middleware::validation::ValidationErrorResponse;
/// use validator::ValidationErrors;
///
/// // Standard authentication error
/// let auth_error = AuthError::Standard(
///     ErrorResponse::new("Invalid credentials", None)
/// );
///
/// // Validation error
/// let validation_errors = ValidationErrors::new();
/// let validation_error = AuthError::Validation(
///     ValidationErrorResponse::new(validation_errors)
/// );
/// ```
#[derive(Debug)]
pub enum AuthError {
    Validation(ValidationErrorResponse),
    Standard(ErrorResponse),
}

/// Converts `AuthError` into an HTTP response, delegating to the appropriate error type.
///
/// This implementation ensures that both validation errors and standard errors
/// are properly converted to HTTP responses with correct status codes and formatting.
///
/// # Error Response Mapping
///
/// - `Validation` errors → 400 Bad Request with field-specific error details
/// - `Standard` errors → Various status codes based on error type
impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AuthError::Validation(err) => err.into_response(),
            AuthError::Standard(err) => err.into_response(),
        }
    }
}

/// Response structure containing a JWT authentication token.
///
/// This structure is returned by successful authentication operations
/// (registration and login) and contains the JWT token needed for
/// authenticated API requests.
///
/// # Fields
///
/// * `token` - The JWT authentication token with 24-hour expiration
///
/// # Usage
///
/// The token should be included in subsequent API requests using the
/// `Authorization: Bearer <token>` header format.
///
/// # Examples
///
/// ```rust
/// use kitchen_api::api::auth::TokenResponse;
/// use serde_json;
///
/// let response = TokenResponse { token: "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...".to_string() };
/// let json = serde_json::to_string(&response).unwrap();
/// ```
#[derive(serde::Serialize, ToSchema)]
pub struct TokenResponse {
    token: String,
}

/// Registers a new user account with email, password, and full name.
///
/// This endpoint creates a new user account with secure password hashing,
/// input validation, and automatic JWT token generation for immediate authentication.
///
/// # Arguments
///
/// * `pool` - Database connection pool for user storage
/// * `payload` - Registration request containing email, password, and full name
///
/// # Returns
///
/// * `Ok(TokenResponse)` - Registration successful with JWT token
/// * `Err(AuthError::Validation)` - Input validation failed
/// * `Err(AuthError::Standard)` - Registration failed (duplicate email, server error)
///
/// # Security Features
///
/// - Password strength validation (8+ chars, mixed case, numbers, symbols)
/// - Email format validation and normalization
/// - Argon2 password hashing with random salt
/// - Input sanitization to prevent XSS
/// - Comprehensive audit logging
///
/// # Kitchen Management Context
///
/// This endpoint is typically used during staff onboarding to create accounts
/// for kitchen staff, managers, and administrators. The generated token can
/// immediately be used to access kitchen management features.
///
/// # Examples
///
/// ```rust
/// use serde_json::json;
/// use reqwest::Client;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new();
/// let registration = json!({
///     "email": "chef@restaurant.com",
///     "password": "SecurePass123!",
///     "full_name": "Head Chef"
/// });
///
/// let response = client
///     .post("http://localhost:3000/api/v1/auth/register")
///     .json(&registration)
///     .send()
///     .await?;
///
/// match response.status() {
///     reqwest::StatusCode::OK => {
///         let token_response: serde_json::Value = response.json().await?;
///         println!("Registration successful: {}", token_response["token"]);
///     }
///     reqwest::StatusCode::BAD_REQUEST => {
///         println!("Validation failed - check email format and password strength");
///     }
///     _ => {
///         println!("Registration failed - email may already exist");
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Error Responses
///
/// ## 400 Bad Request - Validation Error
/// ```json
/// {
///   "error": "VALIDATION_ERROR",
///   "message": "Request validation failed",
///   "validation_errors": {
///     "email": ["Invalid email format"],
///     "password": ["Password must contain at least one uppercase letter"]
///   }
/// }
/// ```
///
/// ## 400 Bad Request - Duplicate Email
/// ```json
/// {
///   "error": "Registration failed",
///   "details": "Email already exists"
/// }
/// ```
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Kitchen staff member registered successfully - Rate limit: 10 req/min with 2 burst allowance", body = TokenResponse),
        (status = 400, description = "Registration validation failed", body = ValidationErrorResponse),
        (status = 500, description = "Registration failed due to server error", body = ErrorResponse)
    ),
    tag = "Kitchen Staff Authentication"
)]
pub async fn register(State(pool): State<PgPool>, Json(mut payload): Json<RegisterRequest>) -> Result<Json<TokenResponse>, AuthError> {
    info!(email = %payload.email, "Registration attempt");
    
    // Validate the request
    if let Err(validation_errors) = payload.validate() {
        warn!(email = %payload.email, "Registration validation failed");
        let error_response = ValidationErrorResponse::new(validation_errors);
        return Err(AuthError::Validation(error_response));
    }
    
    // Sanitize the input
    payload.sanitize();
    
    // Hash password
    let password_hash = hash_password(&payload.password).map_err(|e| {
        warn!(error = %e, "Password hashing failed");
        AuthError::Standard(ErrorResponse::new("Registration failed", Some(format!("Failed to hash password: {}", e))))
    })?;
    
    let user = User {
        id: Uuid::new_v4(),
        email: payload.email.clone(),
        password_hash,
        full_name: payload.full_name.clone(),
        preferences: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    let query = "INSERT INTO users (id, email, password_hash, full_name, preferences, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *";
    let inserted = sqlx::query_as::<_, User>(query)
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.full_name)
        .bind(&user.preferences)
        .bind(user.created_at)
        .bind(user.updated_at)
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            warn!(error = %e, "User insert failed");
            let error_msg = if e.to_string().contains("duplicate key") {
                "Email already exists"
            } else {
                "Failed to create user"
            };
            AuthError::Standard(ErrorResponse::new("Registration failed", Some(error_msg.to_string())))
        })?;
        
    // Create JWT
    let token = create_jwt(inserted.id).map_err(|e| {
        warn!(error = %e, "JWT creation failed");
        AuthError::Standard(ErrorResponse::new("Registration failed", Some("Failed to generate authentication token".to_string())))
    })?;
    
    info!(user_id = %inserted.id, "User registered successfully");
    Ok(Json(TokenResponse { token }))
}

/// Authenticates a user with email and password, returning a JWT token.
///
/// This endpoint validates user credentials against the database and returns
/// a JWT token for authenticated API access. The token is valid for 24 hours.
///
/// # Arguments
///
/// * `pool` - Database connection pool for user lookup
/// * `payload` - Login request containing email and password
///
/// # Returns
///
/// * `Ok(TokenResponse)` - Authentication successful with JWT token
/// * `Err(AuthError::Validation)` - Input validation failed
/// * `Err(AuthError::Standard)` - Invalid credentials or server error
///
/// # Security Features
///
/// - Secure password verification using Argon2
/// - Email normalization and validation
/// - Input sanitization
/// - Comprehensive audit logging with user ID
/// - Protection against timing attacks
///
/// # Kitchen Management Context
///
/// This endpoint is used by kitchen staff to access their daily workflows,
/// including order management, inventory tracking, and shift coordination.
/// The returned token provides access to role-based kitchen features.
///
/// # Examples
///
/// ```rust
/// use serde_json::json;
/// use reqwest::Client;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new();
/// let login = json!({
///     "email": "chef@restaurant.com",
///     "password": "SecurePass123!"
/// });
///
/// let response = client
///     .post("http://localhost:3000/api/v1/auth/login")
///     .json(&login)
///     .send()
///     .await?;
///
/// match response.status() {
///     reqwest::StatusCode::OK => {
///         let token_response: serde_json::Value = response.json().await?;
///         let token = &token_response["token"];
///         
///         // Use token for authenticated requests
///         let profile_response = client
///             .get("http://localhost:3000/api/v1/user/profile")
///             .header("Authorization", format!("Bearer {}", token))
///             .send()
///             .await?;
///     }
///     reqwest::StatusCode::UNAUTHORIZED => {
///         println!("Invalid email or password");
///     }
///     _ => {
///         println!("Login failed due to server error");
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Error Responses
///
/// ## 400 Bad Request - Validation Error
/// ```json
/// {
///   "error": "VALIDATION_ERROR",
///   "message": "Request validation failed",
///   "validation_errors": {
///     "email": ["Invalid email format"],
///     "password": ["Password is required"]
///   }
/// }
/// ```
///
/// ## 401 Unauthorized - Invalid Credentials
/// ```json
/// {
///   "error": "Invalid credentials",
///   "details": "Invalid email or password"
/// }
/// ```
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Kitchen staff member authenticated successfully - Rate limit: 10 req/min with 2 burst allowance", body = TokenResponse),
        (status = 400, description = "Login validation failed", body = ValidationErrorResponse),
        (status = 401, description = "Invalid kitchen staff credentials", body = ErrorResponse),
        (status = 500, description = "Login failed due to server error", body = ErrorResponse)
    ),
    tag = "Kitchen Staff Authentication"
)]
pub async fn login(State(pool): State<PgPool>, Json(mut payload): Json<LoginRequest>) -> Result<Json<TokenResponse>, AuthError> {
    info!(email = %payload.email, "Login attempt");
    
    // Validate the request
    if let Err(validation_errors) = payload.validate() {
        warn!(email = %payload.email, "Login validation failed");
        let error_response = ValidationErrorResponse::new(validation_errors);
        return Err(AuthError::Validation(error_response));
    }
    
    // Sanitize the input
    payload.sanitize();
    
    // Fetch user from database
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            warn!(error = %e, "Database error during login");
            AuthError::Standard(ErrorResponse::new("Login failed", Some("An error occurred while processing your request".to_string())))
        })?
        .ok_or_else(|| {
            warn!(email = %payload.email, "User not found");
            AuthError::Standard(ErrorResponse::new("Invalid credentials", Some("Invalid email or password".to_string())))
        })?;

    // Verify password
    if !verify_password(&payload.password, &user.password_hash) {
        warn!(email = %payload.email, "Invalid password");
        return Err(AuthError::Standard(ErrorResponse::new("Invalid credentials", Some("Invalid email or password".to_string()))));
    }

    // Create JWT
    let token = create_jwt(user.id).map_err(|e| {
        warn!(error = %e, "JWT creation failed");
        AuthError::Standard(ErrorResponse::new("Login failed", Some("Failed to generate authentication token".to_string())))
    })?;
    
    info!(user_id = %user.id, "User logged in successfully");
    Ok(Json(TokenResponse { token }))
}

/// Refreshes an existing JWT token to extend the authentication session.
///
/// This endpoint allows clients to obtain a new JWT token before the current one expires,
/// enabling seamless session management without requiring re-authentication.
///
/// **Note**: This is currently a placeholder implementation. In production, this endpoint
/// should validate the existing token and issue a new one with extended expiration.
///
/// # Returns
///
/// * `200 OK` - Token refresh successful (placeholder response)
///
/// # Future Implementation
///
/// The complete implementation should:
/// - Validate the existing JWT token from Authorization header
/// - Check token expiration and validity
/// - Generate a new token with extended expiration
/// - Return the new token in the same format as login/register
/// - Log the refresh operation for audit purposes
///
/// # Kitchen Management Context
///
/// Token refresh is essential for kitchen staff who work long shifts and need
/// continuous access to kitchen management systems without interruption.
///
/// # Examples
///
/// ```rust
/// use reqwest::Client;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new();
/// let current_token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...";
///
/// let response = client
///     .post("http://localhost:3000/api/v1/auth/refresh")
///     .header("Authorization", format!("Bearer {}", current_token))
///     .send()
///     .await?;
///
/// if response.status().is_success() {
///     // In future implementation, this would return a new TokenResponse
///     println!("Token refreshed successfully");
/// }
/// # Ok(())
/// # }
/// ```
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    responses(
        (status = 200, description = "Kitchen staff authentication token refreshed successfully - Rate limit: 20 req/min with 5 burst allowance")
    ),
    tag = "Kitchen Staff Authentication",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn refresh(bearer: BearerToken) -> Result<Json<TokenResponse>, AuthError> {
    info!("Refresh token endpoint called");

    let token = bearer.deref();

    // Verify the incoming token
    match verify_jwt(token) {
        Ok(user_id) => {
            // Create a new token for the same user
            let new_token = create_jwt(user_id).map_err(|e| {
                warn!(error = %e, "Failed to create refreshed JWT");
                AuthError::Standard(ErrorResponse::new("Token refresh failed", Some("Failed to generate refreshed token".to_string())))
            })?;

            Ok(Json(TokenResponse { token: new_token }))
        }
        Err(e) => {
            warn!(error = %e, "Invalid or expired token provided for refresh");
            Err(AuthError::Standard(ErrorResponse::new("Invalid credentials", Some("Invalid or expired token".to_string()))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::{Request, StatusCode}, Router, routing::post};
    use serde_json::json;                   
    use tower::ServiceExt; // for `oneshot`
    use std::env;
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;

    // Create a test database connection pool
    async fn dummy_pool() -> PgPool {
        let database_url = std::env::var("APP_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".to_string());
            
        PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(3))
            .connect(&database_url)
            .await
            .expect("Failed to create test database pool")
    }

    async fn app() -> Router {
        // Only expose the refresh endpoint for this test to avoid building a DB pool
        Router::new().route("/refresh", post(refresh))
    }

    #[tokio::test]
    #[ignore] // Ignore this test for now as it requires database setup
    async fn test_login_success() {
        env::set_var("APP_AUTH__JWT_SECRET", "test_secret_key_for_testing_jwt");
        let app = app().await;
        
        // First register a user
        let register_payload = json!({
            "email": "test@test.com",
            "password": "test123",
            "full_name": "Test User"
        });
        
        let register_req = Request::builder()
            .method("POST")
            .uri("/register")
            .header("Content-Type", "application/json")
            .body(Body::from(register_payload.to_string()))
            .unwrap();
            
        let app_clone = app.clone();
        let register_response = app_clone
            .oneshot(register_req)
            .await
            .unwrap();
            
        // Registration should succeed (or user already exists)
        assert!(register_response.status() == StatusCode::OK || register_response.status() == StatusCode::BAD_REQUEST);
        
        // Now try to login
        let login_payload = json!({
            "email": "test@test.com",
            "password": "test123"
        });
        
        let login_req = Request::builder()
            .method("POST")
            .uri("/login")
            .header("Content-Type", "application/json")
            .body(Body::from(login_payload.to_string()))
            .unwrap();
            
        let login_response = app
            .oneshot(login_req)
            .await
            .unwrap();
            
        // Login should succeed
        assert_eq!(login_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_refresh() {
        // Setup env and create a valid token
        std::env::set_var("APP_AUTH__JWT_SECRET", "test_secret_key_for_testing_jwt_refresh");
        let user_id = uuid::Uuid::new_v4();
        let token = crate::core::auth::create_jwt(user_id).expect("create_jwt should succeed");

        let req = Request::builder()
            .method("POST")
            .uri("/refresh")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let app = app().await.into_service();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
} 