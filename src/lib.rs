use axum::{Router, routing::{get, post, put, delete}, http::Method, middleware::from_fn};
use sqlx::PgPool;
use tower_http::{trace::TraceLayer, cors::{CorsLayer, Any}};
// use utoipa_swagger_ui::SwaggerUi; // TODO: Re-enable when Swagger UI integration is fixed
use utoipa::OpenApi;
use tonic::transport::Server;
use std::net::SocketAddr;
mod docs;

pub mod config;
pub mod api;
pub mod core;
pub mod infrastructure;
pub mod middleware;
pub mod grpc;

use crate::middleware::rate_limit_configs::RateLimitConfigs;
use crate::middleware::validation::validate_json_middleware;

pub fn app(pool: PgPool) -> Router {
    // Create OpenAPI documentation
    let _openapi = docs::ApiDoc::openapi();
    
    // Create rate limiters
    let auth_rate_limiter = RateLimitConfigs::auth_endpoints();
    let api_rate_limiter = RateLimitConfigs::api_endpoints();
    let public_rate_limiter = RateLimitConfigs::public_endpoints();
    let registration_rate_limiter = RateLimitConfigs::registration();
    
    // Health endpoints with public rate limiting
    let health_router = Router::new()
        .route("/health/live", get(api::health::live))
        .route("/health/ready", get(api::health::ready))
        .layer(from_fn(move |req, next| {
            let limiter = public_rate_limiter.clone();
            async move { limiter.middleware(req, next).await }
        }));
    
    // Registration endpoint with strict rate limiting and validation
    let registration_router = Router::new()
        .route("/api/v1/auth/register", post(api::auth::register))
        .layer(from_fn(validate_json_middleware))
        .layer(from_fn(move |req, next| {
            let limiter = registration_rate_limiter.clone();
            async move { limiter.middleware(req, next).await }
        }));
    
    // Auth endpoints with auth rate limiting and validation
    let auth_router = Router::new()
        .route("/api/v1/auth/login", post(api::auth::login))
        .route("/api/v1/auth/refresh", post(api::auth::refresh))
        .layer(from_fn(validate_json_middleware))
        .layer(from_fn(move |req, next| {
            let limiter = auth_rate_limiter.clone();
            async move { limiter.middleware(req, next).await }
        }));
    
    // API endpoints with moderate rate limiting and validation
    let api_router = Router::new()
        .route("/api/v1/users", post(api::user::create_user))
        .route("/api/v1/users/me", get(api::user::get_current_user))
        .route("/api/v1/users/me/stats", get(api::user::get_current_user_stats))
        .route("/api/v1/users/:id", get(api::user::get_user))
        .route("/api/v1/users/:id", put(api::user::update_user))
        .route("/api/v1/users/:id", delete(api::user::delete_user))
        .route("/api/v1/refresh_tokens", post(api::refresh_token::create_refresh_token))
        .route("/api/v1/refresh_tokens/:id", get(api::refresh_token::get_refresh_token))
        .route("/api/v1/refresh_tokens/:id", put(api::refresh_token::update_refresh_token))
        .route("/api/v1/refresh_tokens/:id", delete(api::refresh_token::delete_refresh_token))
        .layer(from_fn(validate_json_middleware))
        .layer(from_fn(move |req, next| {
            let limiter = api_rate_limiter.clone();
            async move { limiter.middleware(req, next).await }
        }));
    
    // Combine all routers
    let app = Router::new()
        .merge(health_router)
        .merge(registration_router)
        .merge(auth_router)
        .merge(api_router)
        .with_state(pool);
    
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_origin(Any)
        .allow_headers(Any);

    // Create the final router with middleware
    // TODO: Add Swagger UI integration - the OpenAPI spec is generated and available
    let app = app
        .layer(TraceLayer::new_for_http())
        .layer(cors);
    app
}

pub async fn grpc_server(pool: PgPool, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    use grpc::user_stats::{user_stats::user_stats_service_server::UserStatsServiceServer, UserStatsServiceImpl};
    use tonic_reflection::server::Builder as ReflectionBuilder;

    let user_stats_service = UserStatsServiceImpl::new(pool);

    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(include_bytes!(concat!(env!("OUT_DIR"), "/user_stats.bin")))
        .build()?;

    tracing::info!("Starting gRPC server on {}", addr);

    Server::builder()
        .add_service(UserStatsServiceServer::new(user_stats_service))
        .add_service(reflection_service)
        .serve(addr)
        .await?;

    Ok(())
}