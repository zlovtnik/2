use axum::{Router, routing::{get, post, put, delete}, http::Method, Extension};
use sqlx::PgPool;
use tower_http::{trace::TraceLayer, cors::{CorsLayer, Any}};
use utoipa_swagger_ui::SwaggerUi;
use utoipa::OpenApi;
use std::sync::Arc;
mod docs;

pub mod config;
pub mod api;
pub mod core;
pub mod infrastructure;
pub mod middleware;

pub fn app(pool: PgPool) -> Router {
    // Create OpenAPI documentation
    let openapi = docs::ApiDoc::openapi();
    
    // Build our application with all routes
    let app = Router::new()
        // Health check endpoints
        .route("/health/live", get(api::health::live))
        .route("/health/ready", get(api::health::ready))
        
        // Authentication endpoints
        .route("/api/v1/auth/register", post(api::auth::register))
        .route("/api/v1/auth/login", post(api::auth::login))
        .route("/api/v1/auth/refresh", post(api::auth::refresh))
        
        // User endpoints
        .route("/api/v1/users", post(api::user::create_user))
        .route("/api/v1/users/me", get(api::user::get_current_user))
        .route("/api/v1/users/:id", get(api::user::get_user))
        .route("/api/v1/users/:id", put(api::user::update_user))
        .route("/api/v1/users/:id", delete(api::user::delete_user))
        
        // Refresh token endpoints
        .route("/api/v1/refresh_tokens", post(api::refresh_token::create_refresh_token))
        .route("/api/v1/refresh_tokens/:id", get(api::refresh_token::get_refresh_token))
        .route("/api/v1/refresh_tokens/:id", put(api::refresh_token::update_refresh_token))
        .route("/api/v1/refresh_tokens/:id", delete(api::refresh_token::delete_refresh_token))
        
        // Add state
        .with_state(pool);
    
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_origin(Any)
        .allow_headers(Any);

    // Create the final router with middleware and Swagger UI
    let swagger_ui = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi);
    
    let app = Router::new()
        .merge(app)
        .layer(TraceLayer::new_for_http())
        .layer(cors);
    app
} 