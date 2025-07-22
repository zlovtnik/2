use tracing::{info, debug};

pub struct Config {
    pub server_port: u16,
}

pub fn load() -> Config {
    info!("Loading application configuration");
    debug!("Configuration loading started");
    
    // TODO: Load from env or file
    let config = Config { server_port: 3000 };
    
    info!(server_port = config.server_port, "Configuration loaded successfully");
    debug!("Using default configuration values");
    
    config
}