pub struct Config {
    pub websocket_url: String,
}

impl Config {
    pub fn new() -> Self {
        Self {
            websocket_url: env!("WEBSOCKET_URL").to_string(),
        }
    }

    pub fn from_env() -> Self {
        Self {
            websocket_url: std::env::var("WEBSOCKET_URL")
                .unwrap_or_else(|_| env!("WEBSOCKET_URL").to_string()),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
