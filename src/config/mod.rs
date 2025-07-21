pub struct Config {
    pub server_port: u16,
}

pub fn load() -> Config {
    // TODO: Load from env or file
    Config { server_port: 3000 }
} 