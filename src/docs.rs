use utoipa::OpenApi;

/// Main API documentation
/// 
/// This struct is used to generate the OpenAPI documentation for the entire API.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Kitchen Management API",
        version = "1.0.0",
        description = "Comprehensive restaurant kitchen management system API providing authentication, user management, health monitoring, and token management capabilities.",
        contact(
            name = "Kitchen Management Team",
            email = "api@kitchenmanagement.com"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "/api/v1", description = "Production API"),
        (url = "http://localhost:3000/api/v1", description = "Development API")
    ),
    paths(
        // Authentication endpoints
        crate::api::auth::register,
        crate::api::auth::login,
        crate::api::auth::refresh,
        
        // User management endpoints
        crate::api::user::create_user,
        crate::api::user::get_user,
        crate::api::user::get_current_user,
        crate::api::user::get_current_user_stats,
        crate::api::user::update_user,
        crate::api::user::delete_user,
        
        // Health check endpoints
        crate::api::health::live,
        crate::api::health::ready,
        
        // Refresh token management endpoints
        crate::api::refresh_token::create_refresh_token,
        crate::api::refresh_token::get_refresh_token,
        crate::api::refresh_token::update_refresh_token,
        crate::api::refresh_token::delete_refresh_token,
    ),
    components(
        schemas(
            // Authentication schemas
            crate::core::auth::RegisterRequest,
            crate::core::auth::LoginRequest,
            crate::api::auth::TokenResponse,
            crate::api::auth::ErrorResponse,
            
            // User schemas
            crate::core::user::User,
            crate::api::user::UserInfoWithStats,
            
            // Health schemas
            crate::api::health::HealthStatus,
            
            // Refresh token schemas
            crate::core::refresh_token::RefreshToken,
            
            // Validation schemas
            crate::middleware::validation::ValidationErrorResponse,
        )
    ),
    tags(
        (name = "authentication", description = "User authentication and authorization endpoints including registration, login, and token refresh"),
        (name = "users", description = "User management operations including profile management, statistics, and CRUD operations"),
        (name = "health", description = "System health monitoring endpoints for liveness and readiness probes"),
        (name = "tokens", description = "Refresh token management for session handling and token lifecycle management"),
    ),
    external_docs(
        url = "/docs",
        description = "Additional API documentation and implementation guides"
    )
)]
pub struct ApiDoc; 