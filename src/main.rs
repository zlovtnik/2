// Main application entry point for Enterprise Rust JWT Backend
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal;
use tracing_subscriber;

use rust_jwt_backend::{app, grpc_server};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    dotenvy::dotenv().ok();
    let config = rust_jwt_backend::config::load();
    let db_url = std::env::var("APP_DATABASE_URL").expect("APP_DATABASE_URL must be set in .env or environment");
    let pool = PgPool::connect_lazy(&db_url).unwrap();
    
    // Clone pool for gRPC server
    let grpc_pool = pool.clone();
    
    // REST API server
    let rest_app = app(pool);
    let rest_addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    tracing::info!("Starting REST API server on {}", rest_addr);
    
    // gRPC server on a different port
    let grpc_addr = SocketAddr::from(([0, 0, 0, 0], config.server_port + 1));
    
    // Start both servers concurrently
    let rest_server = async {
        let listener = match TcpListener::bind(rest_addr).await {
            Ok(listener) => listener,
            Err(e) => {
                tracing::error!("Failed to bind REST server to {}: {}", rest_addr, e);
                if e.kind() == std::io::ErrorKind::AddrInUse {
                    tracing::error!("Port {} is already in use. Please ensure no other instance is running or use a different port.", rest_addr.port());
                }
                std::process::exit(1);
            }
        };
        
        if let Err(e) = axum::serve(listener, rest_app.into_make_service()).await {
            tracing::error!("REST server error: {}", e);
        }
    };
    
    let grpc_server_task = async {
        if let Err(e) = grpc_server(grpc_pool, grpc_addr).await {
            tracing::error!("gRPC server error: {}", e);
            // Check if it's a port binding issue
            if e.to_string().contains("Address already in use") || e.to_string().contains("AddrInUse") {
                tracing::error!("Port {} is already in use for gRPC server. Please ensure no other instance is running or use a different port.", grpc_addr.port());
            }
        }
    };
    
    // Create shutdown signal handler
    let shutdown_signal = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        tracing::info!("Received shutdown signal, gracefully shutting down...");
    };

    // Run both servers concurrently with graceful shutdown
    tokio::select! {
        _ = rest_server => {
            tracing::info!("REST server finished");
        }
        _ = grpc_server_task => {
            tracing::info!("gRPC server finished");
        }
        _ = shutdown_signal => {
            tracing::info!("Shutdown signal received, terminating servers...");
        }
    }
    
    tracing::info!("Application shutdown complete");
}