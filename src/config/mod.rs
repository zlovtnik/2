use tracing::{info, debug};

pub struct Config {
    pub server_port: u16,
}

pub fn load() -> Config {
    info!("Loading application configuration");
    debug!("Configuration loading started");
    
    // Load server port from environment variable or use default
    let server_port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    
    let config = Config { server_port };
    
    info!(server_port = config.server_port, "Configuration loaded successfully");
    debug!("Using port from environment or default value");
    
    config
}