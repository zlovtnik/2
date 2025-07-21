use axum::{Router, routing::{get, post}};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

pub mod config;
pub mod api;
pub mod core;
pub mod infrastructure;
pub mod middleware;

pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/health/live", get(api::health::live))
        .route("/health/ready", get(api::health::ready))
        .route("/api/v1/auth/register", post(api::auth::register))
        .route("/api/v1/auth/login", post(api::auth::login))
        .route("/api/v1/auth/refresh", post(api::auth::refresh))
        // User CRUD
        .route("/api/v1/users", post(api::user::create_user))
        .route("/api/v1/users/{id}", axum::routing::get(api::user::get_user))
        .route("/api/v1/users/{id}", axum::routing::delete(api::user::delete_user))
        .route("/api/v1/users/{id}", axum::routing::put(api::user::update_user))
        // Refresh token CRUD
        .route("/api/v1/refresh_tokens", post(api::refresh_token::create_refresh_token))
        .route("/api/v1/refresh_tokens/{id}", axum::routing::get(api::refresh_token::get_refresh_token))
        .route("/api/v1/refresh_tokens/{id}", axum::routing::delete(api::refresh_token::delete_refresh_token))
        .route("/api/v1/refresh_tokens/{id}", axum::routing::put(api::refresh_token::update_refresh_token))
        .layer(TraceLayer::new_for_http())
        .with_state(pool)
} 