//! User management API endpoints for CRUD operations and profile management.
//!
//! This module provides comprehensive user management functionality including:
//! - User creation and profile management
//! - User lookup and statistics
//! - Authentication-protected operations
//! - Integration with PostgreSQL stored procedures
//!
//! # Authentication
//!
//! Most endpoints require JWT authentication via the `Authorization: Bearer <token>` header.
//! The token is validated using the `AuthenticatedUser` extractor.
//!
//! # Kitchen Management Context
//!
//! These endpoints support kitchen staff management, including:
//! - Staff profile management
//! - Role-based access control
//! - Activity tracking and statistics
//! - Shift management integration
//!
//! # Examples
//!
//! ## Get Current User Profile
//!
//! ```rust
//! use reqwest::Client;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new();
//! let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...";
//!
//! let response = client
//!     .get("http://localhost:3000/api/v1/user/profile")
//!     .header("Authorization", format!("Bearer {}", token))
//!     .send()
//!     .await?;
//!
//! if response.status().is_success() {
//!     let user: serde_json::Value = response.json().await?;
//!     println!("User: {} ({})", user["full_name"], user["email"]);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Get User Statistics
//!
//! ```rust
//! use reqwest::Client;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new();
//! let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...";
//!
//! let response = client
//!     .get("http://localhost:3000/api/v1/user/stats")
//!     .header("Authorization", format!("Bearer {}", token))
//!     .send()
//!     .await?;
//!
//! if response.status().is_success() {
//!     let stats: serde_json::Value = response.json().await?;
//!     println!("Refresh tokens: {}", stats["refresh_token_count"]);
//!     println!("Last login: {:?}", stats["last_login"]);
//! }
//! # Ok(())
//! # }
//! ```

use axum::{Json, extract::{Path, State}, response::IntoResponse};
use uuid::Uuid;
use crate::core::user::User;
use crate::infrastructure::database::{Crud, PgCrud, UpdatableCrud};
use sqlx::{PgPool, FromRow};
use axum::http::StatusCode;
use crate::middleware::auth::AuthenticatedUser;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Extended user information structure that includes usage statistics and activity data.
///
/// This structure combines basic user profile information with statistical data
/// retrieved from PostgreSQL stored procedures. It provides comprehensive insights
/// into user activity and system usage patterns.
///
/// # Fields
///
/// * `user_id` - Unique identifier for the user
/// * `email` - User's email address (normalized)
/// * `full_name` - User's display name
/// * `preferences` - JSON object containing user preferences and settings
/// * `created_at` - Account creation timestamp
/// * `updated_at` - Last profile modification timestamp
/// * `refresh_token_count` - Number of active refresh tokens for this user
/// * `last_login` - Timestamp of the user's most recent login (if available)
///
/// # Kitchen Management Context
///
/// This structure is particularly useful for:
/// - Staff activity monitoring
/// - Shift pattern analysis
/// - Security auditing (multiple active sessions)
/// - User engagement metrics
///
/// # Examples
///
/// ```rust
/// use kitchen_api::api::user::UserInfoWithStats;
/// use uuid::Uuid;
/// use chrono::Utc;
///
/// let user_stats = UserInfoWithStats {
///     user_id: Uuid::new_v4(),
///     email: "chef@restaurant.com".to_string(),
///     full_name: "Head Chef".to_string(),
///     preferences: Some(serde_json::json!({
///         "theme": "dark",
///         "notifications": true
///     })),
///     created_at: Utc::now(),
///     updated_at: Utc::now(),
///     refresh_token_count: 2,
///     last_login: Some(Utc::now()),
/// };
///
/// println!("User {} has {} active sessions", user_stats.full_name, user_stats.refresh_token_count);
/// ```
#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UserInfoWithStats {
    pub user_id: Uuid,
    pub email: String,
    pub full_name: String,
    pub preferences: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub refresh_token_count: i64,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
}

/// Creates a boxed CRUD implementation for User operations.
///
/// This helper function creates a trait object that implements the `Crud` trait
/// for `User` entities, providing a consistent interface for database operations.
///
/// # Arguments
///
/// * `pool` - Database connection pool
///
/// # Returns
///
/// A boxed trait object implementing `Crud<User, Uuid>` for database operations.
///
/// # Examples
///
/// ```rust
/// use sqlx::PgPool;
/// use uuid::Uuid;
///
/// # async fn example(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
/// let crud = user_crud_box(pool);
/// let user_id = Uuid::new_v4();
/// 
/// match crud.read(user_id).await {
///     Ok(Some(user)) => println!("Found user: {}", user.email),
///     Ok(None) => println!("User not found"),
///     Err(e) => println!("Database error: {}", e),
/// }
/// # Ok(())
/// # }
/// ```
fn user_crud_box(pool: PgPool) -> Box<dyn Crud<User, Uuid> + Send + Sync> {
    Box::new(PgCrud::new(pool, "users"))
}

/// Creates a new user account in the system.
///
/// This endpoint creates a new user with the provided information. It performs
/// direct database insertion with comprehensive error handling and logging.
///
/// # Arguments
///
/// * `pool` - Database connection pool for user storage
/// * `user` - Complete user object with all required fields
///
/// # Returns
///
/// * `201 Created` with user data - User created successfully
/// * `409 Conflict` - User with email already exists
/// * `500 Internal Server Error` - Database or server error
///
/// # Security Considerations
///
/// - This endpoint does not require authentication (for admin/system use)
/// - Password should already be hashed before calling this endpoint
/// - Email uniqueness is enforced at the database level
///
/// # Kitchen Management Context
///
/// This endpoint is typically used by administrators to create staff accounts
/// or by automated systems during bulk user imports.
///
/// # Examples
///
/// ```rust
/// use reqwest::Client;
/// use serde_json::json;
/// use uuid::Uuid;
/// use chrono::Utc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new();
/// let user_data = json!({
///     "id": Uuid::new_v4(),
///     "email": "newstaff@restaurant.com",
///     "password_hash": "$argon2id$v=19$m=4096,t=3,p=1$...", // Pre-hashed password
///     "full_name": "New Kitchen Staff",
///     "preferences": null,
///     "created_at": Utc::now(),
///     "updated_at": Utc::now()
/// });
///
/// let response = client
///     .post("http://localhost:3000/api/v1/users")
///     .json(&user_data)
///     .send()
///     .await?;
///
/// match response.status() {
///     reqwest::StatusCode::CREATED => {
///         let created_user: serde_json::Value = response.json().await?;
///         println!("User created: {}", created_user["email"]);
///     }
///     reqwest::StatusCode::CONFLICT => {
///         println!("User with this email already exists");
///     }
///     _ => {
///         println!("Failed to create user");
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Error Responses
///
/// ## 409 Conflict - Duplicate Email
/// ```text
/// User with this email already exists
/// ```
///
/// ## 500 Internal Server Error - Database Error
/// ```text
/// DB error: connection refused
/// ```
#[utoipa::path(
    post,
    path = "/api/v1/users",
    request_body = User,
    responses(
        (status = 201, description = "User created successfully", body = User),
        (status = 409, description = "User with email already exists"),
        (status = 500, description = "Database error")
    ),
    tag = "users"
)]
pub async fn create_user(State(pool): State<PgPool>, Json(user): Json<User>) -> impl IntoResponse {
    info!(user_id = %user.id, email = %user.email, "Creating new user");
    debug!(user_id = %user.id, full_name = %user.full_name, "User creation details");
    
    // Direct SQLx for insert (as before)
    let query = "INSERT INTO users (id, email, password_hash, full_name, preferences, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *";
    debug!("Executing user insert query");
    
    match sqlx::query_as::<_, User>(query)
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.full_name)
        .bind(&user.preferences)
        .bind(user.created_at)
        .bind(user.updated_at)
        .fetch_one(&pool)
        .await
    {
        Ok(inserted) => {
            info!(user_id = %inserted.id, "User created successfully");
            (StatusCode::CREATED, Json(inserted)).into_response()
        },
        Err(e) => {
            error!(user_id = %user.id, error = %e, "Failed to create user");
            if e.to_string().contains("duplicate key") {
                warn!(email = %user.email, "Attempted to create user with duplicate email");
                (StatusCode::CONFLICT, "User with this email already exists".to_string()).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response()
            }
        },
    }
}

/// Retrieves a specific user by ID with authentication context.
///
/// This endpoint fetches user information by UUID and returns both the requested
/// user data and the authenticated user's ID for audit and authorization purposes.
///
/// # Arguments
///
/// * `pool` - Database connection pool for user lookup
/// * `id` - UUID of the user to retrieve
/// * `user_id` - Authenticated user's ID (from JWT token)
///
/// # Returns
///
/// * `200 OK` with user data and authenticated user ID - User found
/// * `404 Not Found` - User with specified ID does not exist
/// * `401 Unauthorized` - Invalid or missing authentication token
/// * `500 Internal Server Error` - Database or server error
///
/// # Authentication
///
/// Requires valid JWT token in `Authorization: Bearer <token>` header.
///
/// # Kitchen Management Context
///
/// This endpoint allows kitchen staff to look up colleague information,
/// useful for shift coordination and task assignment.
///
/// # Examples
///
/// ```rust
/// use reqwest::Client;
/// use uuid::Uuid;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new();
/// let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...";
/// let user_id = Uuid::new_v4();
///
/// let response = client
///     .get(&format!("http://localhost:3000/api/v1/users/{}", user_id))
///     .header("Authorization", format!("Bearer {}", token))
///     .send()
///     .await?;
///
/// match response.status() {
///     reqwest::StatusCode::OK => {
///         let (user, authenticated_user_id): (serde_json::Value, Uuid) = response.json().await?;
///         println!("Found user: {} (requested by: {})", user["email"], authenticated_user_id);
///     }
///     reqwest::StatusCode::NOT_FOUND => {
///         println!("User not found");
///     }
///     reqwest::StatusCode::UNAUTHORIZED => {
///         println!("Authentication required");
///     }
///     _ => {
///         println!("Request failed");
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Response Format
///
/// Returns a tuple containing:
/// 1. The requested user object
/// 2. The authenticated user's UUID
///
/// ```json
/// [
///   {
///     "id": "123e4567-e89b-12d3-a456-426614174000",
///     "email": "staff@restaurant.com",
///     "full_name": "Kitchen Staff",
///     "preferences": null,
///     "created_at": "2024-01-01T00:00:00Z",
///     "updated_at": "2024-01-01T00:00:00Z"
///   },
///   "987fcdeb-51a2-43d7-8f9e-123456789abc"
/// ]
/// ```
#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    params(
        ("id" = Uuid, Path, description = "User ID to retrieve")
    ),
    responses(
        (status = 200, description = "User found"),
        (status = 404, description = "User not found"),
        (status = 401, description = "Authentication required"),
        (status = 500, description = "Database error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "users"
)]
pub async fn get_user(State(pool): State<PgPool>, Path(id): Path<Uuid>, AuthenticatedUser(user_id): AuthenticatedUser) -> impl IntoResponse {
    info!(requested_user_id = %id, authenticated_user_id = %user_id, "Getting user");
    debug!("Creating user CRUD instance");
    
    let crud = user_crud_box(pool);
    debug!(user_id = %id, "Executing user read query");
    
    // For demo: return the authenticated user_id and the requested user
    match crud.read(id).await {
        Ok(Some(user)) => {
            info!(user_id = %id, authenticated_user_id = %user_id, "User retrieved successfully");
            debug!(user_email = %user.email, "User details retrieved");
            (StatusCode::OK, Json((user, user_id))).into_response()
        },
        Ok(None) => {
            warn!(user_id = %id, authenticated_user_id = %user_id, "User not found");
            (StatusCode::NOT_FOUND, "Not found").into_response()
        },
        Err(e) => {
            error!(user_id = %id, authenticated_user_id = %user_id, error = %e, "Failed to retrieve user");
            (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response()
        },
    }
}

pub async fn delete_user(State(pool): State<PgPool>, Path(id): Path<Uuid>) -> impl IntoResponse {
    info!(user_id = %id, "Deleting user");
    debug!("Creating user CRUD instance for deletion");
    
    let crud = user_crud_box(pool);
    debug!(user_id = %id, "Executing user delete query");
    
    match crud.delete(id).await {
        Ok(affected) if affected > 0 => {
            info!(user_id = %id, affected_rows = affected, "User deleted successfully");
            (StatusCode::NO_CONTENT, "").into_response()
        },
        Ok(affected) => {
            warn!(user_id = %id, affected_rows = affected, "User not found for deletion");
            (StatusCode::NOT_FOUND, "Not found").into_response()
        },
        Err(e) => {
            error!(user_id = %id, error = %e, "Failed to delete user");
            (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response()
        },
    }
}

/// Retrieves the current authenticated user's profile information.
///
/// This endpoint returns the complete profile of the user making the request,
/// identified by the JWT token. It's commonly used for profile pages and
/// user context in applications.
///
/// # Arguments
///
/// * `user_id` - Authenticated user's ID extracted from JWT token
/// * `pool` - Database connection pool for user lookup
///
/// # Returns
///
/// * `200 OK` with user profile data - Profile retrieved successfully
/// * `404 Not Found` - Authenticated user not found in database (token/DB mismatch)
/// * `401 Unauthorized` - Invalid or missing authentication token
/// * `500 Internal Server Error` - Database or server error
///
/// # Authentication
///
/// Requires valid JWT token in `Authorization: Bearer <token>` header.
/// The user ID is automatically extracted from the token.
///
/// # Kitchen Management Context
///
/// This endpoint is essential for displaying user information in the kitchen
/// management interface, including staff name, preferences, and role information.
///
/// # Examples
///
/// ```rust
/// use reqwest::Client;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new();
/// let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...";
///
/// let response = client
///     .get("http://localhost:3000/api/v1/user/profile")
///     .header("Authorization", format!("Bearer {}", token))
///     .send()
///     .await?;
///
/// match response.status() {
///     reqwest::StatusCode::OK => {
///         let user: serde_json::Value = response.json().await?;
///         println!("Welcome, {}!", user["full_name"]);
///         println!("Email: {}", user["email"]);
///         
///         if let Some(preferences) = user["preferences"].as_object() {
///             println!("Theme: {:?}", preferences.get("theme"));
///         }
///     }
///     reqwest::StatusCode::NOT_FOUND => {
///         println!("User profile not found - token may be invalid");
///     }
///     reqwest::StatusCode::UNAUTHORIZED => {
///         println!("Authentication required");
///     }
///     _ => {
///         println!("Failed to retrieve profile");
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Response Format
///
/// ```json
/// {
///   "id": "123e4567-e89b-12d3-a456-426614174000",
///   "email": "chef@restaurant.com",
///   "full_name": "Head Chef",
///   "preferences": {
///     "theme": "dark",
///     "notifications": true,
///     "language": "en"
///   },
///   "created_at": "2024-01-01T00:00:00Z",
///   "updated_at": "2024-01-15T10:30:00Z"
/// }
/// ```
pub async fn get_current_user(AuthenticatedUser(user_id): AuthenticatedUser, State(pool): State<PgPool>) -> impl IntoResponse {
    info!(user_id = %user_id, "Getting current user profile");
    debug!("Creating user CRUD instance for current user");
    
    let crud = user_crud_box(pool);
    debug!(user_id = %user_id, "Executing current user read query");
    
    match crud.read(user_id).await {
        Ok(Some(user)) => {
            info!(user_id = %user_id, "Current user profile retrieved successfully");
            debug!(user_email = %user.email, full_name = %user.full_name, "Current user details");
            (StatusCode::OK, Json(user)).into_response()
        },
        Ok(None) => {
            warn!(user_id = %user_id, "Current user not found in database");
            (StatusCode::NOT_FOUND, "User not found").into_response()
        },
        Err(e) => {
            error!(user_id = %user_id, error = %e, "Failed to retrieve current user");
            (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response()
        },
    }
}

/// Retrieves the current user's profile with comprehensive usage statistics.
///
/// This endpoint calls a PostgreSQL stored procedure to fetch extended user information
/// including activity statistics, session data, and usage patterns. It demonstrates
/// integration with database procedures for complex data aggregation.
///
/// # Arguments
///
/// * `user_id` - Authenticated user's ID extracted from JWT token
/// * `pool` - Database connection pool for procedure execution
///
/// # Returns
///
/// * `200 OK` with extended user statistics - Data retrieved successfully
/// * `404 Not Found` - User not found or procedure failed
/// * `401 Unauthorized` - Invalid or missing authentication token
/// * `500 Internal Server Error` - Database procedure error
///
/// # Database Procedure
///
/// Calls the `get_user_info_with_stats($1)` PostgreSQL procedure which:
/// - Joins user data with refresh token statistics
/// - Calculates login patterns and activity metrics
/// - Provides comprehensive user insights
///
/// # Authentication
///
/// Requires valid JWT token in `Authorization: Bearer <token>` header.
///
/// # Kitchen Management Context
///
/// This endpoint provides managers with detailed staff activity insights:
/// - Session management (multiple device logins)
/// - Usage patterns for shift planning
/// - Security monitoring (unusual login activity)
/// - Staff engagement metrics
///
/// # Examples
///
/// ```rust
/// use reqwest::Client;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new();
/// let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...";
///
/// let response = client
///     .get("http://localhost:3000/api/v1/user/stats")
///     .header("Authorization", format!("Bearer {}", token))
///     .send()
///     .await?;
///
/// match response.status() {
///     reqwest::StatusCode::OK => {
///         let stats: serde_json::Value = response.json().await?;
///         
///         println!("User: {} ({})", stats["full_name"], stats["email"]);
///         println!("Active sessions: {}", stats["refresh_token_count"]);
///         
///         if let Some(last_login) = stats["last_login"].as_str() {
///             println!("Last login: {}", last_login);
///         } else {
///             println!("No previous login recorded");
///         }
///         
///         println!("Account created: {}", stats["created_at"]);
///     }
///     reqwest::StatusCode::NOT_FOUND => {
///         println!("User statistics not available");
///     }
///     reqwest::StatusCode::UNAUTHORIZED => {
///         println!("Authentication required");
///     }
///     _ => {
///         println!("Failed to retrieve statistics");
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Response Format
///
/// ```json
/// {
///   "user_id": "123e4567-e89b-12d3-a456-426614174000",
///   "email": "chef@restaurant.com",
///   "full_name": "Head Chef",
///   "preferences": {
///     "theme": "dark",
///     "notifications": true
///   },
///   "created_at": "2024-01-01T00:00:00Z",
///   "updated_at": "2024-01-15T10:30:00Z",
///   "refresh_token_count": 3,
///   "last_login": "2024-01-15T08:45:00Z"
/// }
/// ```
///
/// # PostgreSQL Procedure Integration
///
/// The procedure `get_user_info_with_stats` performs:
/// ```sql
/// CREATE OR REPLACE FUNCTION get_user_info_with_stats(user_uuid UUID)
/// RETURNS TABLE (
///     user_id UUID,
///     email TEXT,
///     full_name TEXT,
///     preferences JSONB,
///     created_at TIMESTAMPTZ,
///     updated_at TIMESTAMPTZ,
///     refresh_token_count BIGINT,
///     last_login TIMESTAMPTZ
/// )
/// ```
pub async fn get_current_user_stats(
    AuthenticatedUser(user_id): AuthenticatedUser, 
    State(pool): State<PgPool>
) -> impl IntoResponse {
    info!(user_id = %user_id, "Getting current user stats via PostgreSQL procedure");
    debug!("Calling get_user_info_with_stats procedure");
    
    // Call the PostgreSQL procedure with the authenticated user's ID
    let query = "SELECT * FROM get_user_info_with_stats($1)";
    
    match sqlx::query_as::<_, UserInfoWithStats>(query)
        .bind(user_id)
        .fetch_one(&pool)
        .await
    {
        Ok(user_stats) => {
            info!(
                user_id = %user_id, 
                email = %user_stats.email,
                refresh_token_count = user_stats.refresh_token_count,
                "User stats retrieved successfully via procedure"
            );
            debug!(
                full_name = %user_stats.full_name,
                created_at = %user_stats.created_at,
                last_login = ?user_stats.last_login,
                "Detailed user stats from procedure"
            );
            (StatusCode::OK, Json(user_stats)).into_response()
        },
        Err(e) => {
            error!(user_id = %user_id, error = %e, "Failed to retrieve user stats via procedure");
            if e.to_string().contains("not found") {
                warn!(user_id = %user_id, "User not found in procedure call");
                (StatusCode::NOT_FOUND, "User not found").into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Procedure error: {}", e)).into_response()
            }
        },
    }
}

pub async fn update_user(State(pool): State<PgPool>, Path(id): Path<Uuid>, Json(new_name): Json<String>) -> impl IntoResponse {
    info!(user_id = %id, new_name = %new_name, "Updating user");
    debug!("Creating user CRUD instance for update");
    
    let crud: PgCrud<User> = PgCrud::new(pool, "users");
    debug!(user_id = %id, "Checking if user exists before update");
    
    match crud.read(id).await {
        Ok(Some(existing)) => {
            info!(user_id = %id, old_name = %existing.full_name, new_name = %new_name, "User found, proceeding with update");
            let update_fn = |mut u: User| {
                u.full_name = new_name.clone();
                u
            };
            debug!(user_id = %id, "Executing user update");
            match UpdatableCrud::update(&crud, id, update_fn).await {
                Ok(Some(updated)) => {
                    info!(user_id = %id, updated_name = %updated.full_name, "User updated successfully");
                    (StatusCode::OK, Json(updated)).into_response()
                },
                Ok(None) => {
                    warn!(user_id = %id, "User not found during update operation");
                    (StatusCode::NOT_FOUND, "Not found").into_response()
                },
                Err(e) => {
                    error!(user_id = %id, error = %e, "Failed to update user");
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response()
                },
            }
        }
        Ok(None) => {
            warn!(user_id = %id, "User not found for update");
            (StatusCode::NOT_FOUND, "Not found").into_response()
        },
        Err(e) => {
            error!(user_id = %id, error = %e, "Failed to check user existence before update");
            (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)).into_response()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::{Request, StatusCode}, Router, routing::post};
    use serde_json::json;
    use tower::ServiceExt; // for `oneshot`
    use sqlx::PgPool;
    
    
    use chrono::Utc;
    use uuid::Uuid;

    // Dummy pool for demonstration (not a real DB connection)
    fn dummy_pool() -> PgPool {
        // This will panic if actually used, but allows us to test endpoint wiring
        PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap()
    }

    fn app() -> Router {
        Router::new()
            .route("/users", post(create_user))
            // Add more routes as needed
            .with_state(dummy_pool())
    }

    #[tokio::test]
    async fn test_create_user_returns_500_on_db_error() {
        let user = json!({
            "id": Uuid::new_v4(),
            "email": "test@example.com",
            "password_hash": "hash",
            "full_name": "Test User",
            "preferences": null,
            "created_at": Utc::now(),
            "updated_at": Utc::now()
        });
        let req = Request::builder()
            .method("POST")
            .uri("/users")
            .header("content-type", "application/json")
            .body(Body::from(user.to_string()))
            .unwrap();
        let res = app().oneshot(req).await.unwrap();
        // Since the dummy pool is not connected, this should return 500
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
} 