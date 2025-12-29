use crate::domain::IceServer;

/// Configuration for P2P session
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Matchbox signalling server URL
    pub signalling_server: String,

    /// Polling interval in milliseconds
    pub poll_interval_ms: u64,

    /// ICE servers for WebRTC connection
    pub ice_servers: Vec<IceServer>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            signalling_server: "wss://match.konnektoren.help".to_string(),
            poll_interval_ms: 100,
            ice_servers: IceServer::default_stun_servers(),
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

    /// Add a STUN server
    pub fn with_stun_server(mut self, url: String) -> Self {
        self.ice_servers.push(IceServer::stun(url));
        self
    }

    /// Add a TURN server with authentication
    pub fn with_turn_server(mut self, url: String, username: String, credential: String) -> Self {
        self.ice_servers
            .push(IceServer::turn(url, username, credential));
        self
    }

    /// Set custom ICE servers (replaces defaults)
    pub fn with_ice_servers(mut self, ice_servers: Vec<IceServer>) -> Self {
        self.ice_servers = ice_servers;
        self
    }

    /// Add additional ICE servers
    pub fn add_ice_servers(mut self, mut ice_servers: Vec<IceServer>) -> Self {
        self.ice_servers.append(&mut ice_servers);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SessionConfig::default();
        assert_eq!(config.signalling_server, "wss://match.konnektoren.help");
        assert_eq!(config.poll_interval_ms, 100);
        assert!(!config.ice_servers.is_empty());
    }

    #[test]
    fn test_with_stun_server() {
        let config =
            SessionConfig::default().with_stun_server("stun:custom.stun.server:3478".to_string());
        assert!(
            config
                .ice_servers
                .iter()
                .any(|s| s.urls.contains(&"stun:custom.stun.server:3478".to_string()))
        );
    }

    #[test]
    fn test_with_turn_server() {
        let config = SessionConfig::default().with_turn_server(
            "turn:turn.example.com:3478".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        assert!(config.ice_servers.iter().any(|s| s.username.is_some()));
    }

    #[test]
    fn test_with_ice_servers() {
        let custom_servers = vec![IceServer::stun("stun:custom.com:3478".to_string())];
        let config = SessionConfig::default().with_ice_servers(custom_servers.clone());
        assert_eq!(config.ice_servers.len(), 1);
        assert_eq!(config.ice_servers[0], custom_servers[0]);
    }
}
