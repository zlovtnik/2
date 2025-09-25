use tracing::{info, debug};

pub struct Config {
    pub server_port: u16,
    pub grpc_connection_pool_size: usize,
    pub grpc_connection_timeout_secs: u64,
    pub grpc_health_check_interval_secs: u64,
}

pub fn load() -> Config {
    info!("Loading application configuration");
    debug!("Configuration loading started");
    
    // Load server port from environment variable or use default
    let server_port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    
    // Load gRPC connection pool configuration
    let grpc_connection_pool_size = std::env::var("GRPC_CONNECTION_POOL_SIZE")
        .ok()
        .and_then(|p| p.parse::<usize>().ok())
        .unwrap_or(10); // Default to 10 connections
    
    let grpc_connection_timeout_secs = std::env::var("GRPC_CONNECTION_TIMEOUT_SECS")
        .ok()
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(30); // Default to 30 seconds
    
    let grpc_health_check_interval_secs = std::env::var("GRPC_HEALTH_CHECK_INTERVAL_SECS")
        .ok()
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(60); // Default to 60 seconds
    
    let config = Config { 
        server_port,
        grpc_connection_pool_size,
        grpc_connection_timeout_secs,
        grpc_health_check_interval_secs,
    };
    
    info!(
        server_port = config.server_port,
        grpc_connection_pool_size = config.grpc_connection_pool_size,
        grpc_connection_timeout_secs = config.grpc_connection_timeout_secs,
        grpc_health_check_interval_secs = config.grpc_health_check_interval_secs,
        "Configuration loaded successfully"
    );
    debug!("Using port and gRPC settings from environment or default values");
    
    config
}