// Main application entry point for Enterprise Rust JWT Backend
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber;

use rust_jwt_backend::app;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    dotenvy::dotenv().ok();
    let config = rust_jwt_backend::config::load();
    let db_url = std::env::var("APP_DATABASE_URL").expect("APP_DATABASE_URL must be set in .env or environment");
    let pool = PgPool::connect_lazy(&db_url).unwrap();
    let app = app(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    tracing::info!("Listening on {}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
} 