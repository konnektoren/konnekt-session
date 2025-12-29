/// Configuration for P2P session
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Matchbox signalling server URL
    pub signalling_server: String,

    /// Polling interval in milliseconds
    pub poll_interval_ms: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            signalling_server: "wss://match.konnektoren.help".to_string(),
            poll_interval_ms: 100,
        }
    }
}

impl SessionConfig {
    pub fn new(signalling_server: String) -> Self {
        Self {
            signalling_server,
            ..Default::default()
        }
    }

    pub fn with_poll_interval(mut self, ms: u64) -> Self {
        self.poll_interval_ms = ms;
        self
    }
}
