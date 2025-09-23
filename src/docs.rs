use utoipa::OpenApi;

pub mod validation;

/// Main API documentation
/// 
/// This struct is used to generate the OpenAPI documentation for the entire API.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Kitchen Management API",
        version = "1.0.0",
        description = "Comprehensive restaurant kitchen management system API providing authentication, user management, health monitoring, and token management capabilities. This API supports kitchen staff workflows including order processing, inventory management, and real-time kitchen operations coordination.",
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
        (url = "/api/v1", description = "Production API - Kitchen Management System"),
        (url = "http://localhost:3000/api/v1", description = "Development API - Local Kitchen Environment")
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
        (
            name = "Kitchen Staff Authentication", 
            description = "Authentication endpoints for kitchen staff including chefs, line cooks, and kitchen managers. Supports secure registration, login, and session management for kitchen operations.",
            external_docs(
                url = "/docs/authentication",
                description = "Kitchen staff authentication guide"
            )
        ),
        (
            name = "Kitchen Staff Management", 
            description = "User management operations for kitchen personnel including profile management, staff statistics, and role-based access control. Essential for managing kitchen team workflows and permissions.",
            external_docs(
                url = "/docs/staff-management", 
                description = "Kitchen staff management workflows"
            )
        ),
        (
            name = "System Health & Monitoring", 
            description = "Health monitoring endpoints for kitchen management system reliability. Used by load balancers and monitoring systems to ensure continuous kitchen operations during peak service hours.",
            external_docs(
                url = "/docs/monitoring",
                description = "System monitoring and alerting guide"
            )
        ),
        (
            name = "Session & Token Management", 
            description = "Refresh token management for extended kitchen shifts and session handling. Supports long-running kitchen operations without authentication interruptions.",
            external_docs(
                url = "/docs/session-management",
                description = "Session management for kitchen workflows"
            )
        ),
    ),
    external_docs(
        url = "/docs",
        description = "Complete kitchen management API documentation including workflow guides, integration examples, and best practices for restaurant operations"
    )
)]
pub struct ApiDoc; 