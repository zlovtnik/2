// Main application entry point for Enterprise Rust JWT Backend
use axum::{routing::get, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber;
use sqlx::PgPool;
use axum::extract::State;

mod config;
mod api;
mod core;
mod infrastructure;
mod middleware;

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Load configuration (stub for now)
    let config = config::load();

    // Build application router
    let app = Router::new()
        .route("/health/live", get(api::health::live))
        .route("/health/ready", get(api::health::ready))
        .route("/api/v1/auth/register", axum::routing::post(api::auth::register))
        .route("/api/v1/auth/login", axum::routing::post(api::auth::login))
        .route("/api/v1/auth/refresh", axum::routing::post(api::auth::refresh))
        // User CRUD
        .route("/api/v1/users", axum::routing::post(api::user::create_user))
        .route("/api/v1/users/{id}", axum::routing::get(api::user::get_user))
        .route("/api/v1/users/{id}", axum::routing::delete(api::user::delete_user))
        .route("/api/v1/users/{id}", axum::routing::put(api::user::update_user))
        // Refresh token CRUD
        .route("/api/v1/refresh_tokens", axum::routing::post(api::refresh_token::create_refresh_token))
        .route("/api/v1/refresh_tokens/{id}", axum::routing::get(api::refresh_token::get_refresh_token))
        .route("/api/v1/refresh_tokens/{id}", axum::routing::delete(api::refresh_token::delete_refresh_token))
        .route("/api/v1/refresh_tokens/{id}", axum::routing::put(api::refresh_token::update_refresh_token))
        .layer(tower_http::trace::TraceLayer::new_for_http());

    // Provide PgPool as state (stub: you should initialize the pool properly)
    dotenvy::dotenv().ok();
    let db_url = std::env::var("APP_DATABASE__URL")
        .expect("APP_DATABASE__URL must be set in .env or environment");
    let pool = PgPool::connect_lazy(&db_url).unwrap();
    let app = app.with_state(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    tracing::info!("Listening on {}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
} 