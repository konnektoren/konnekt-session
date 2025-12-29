use serde::{Deserialize, Serialize};

/// ICE server configuration for WebRTC
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IceServer {
    /// Server URLs (can have multiple for failover)
    pub urls: Vec<String>,
    /// Username for authentication (optional, required for TURN)
    pub username: Option<String>,
    /// Credential for authentication (optional, required for TURN)
    pub credential: Option<String>,
}

impl IceServer {
    /// Create a STUN server configuration
    pub fn stun(url: String) -> Self {
        Self {
            urls: vec![url],
            username: None,
            credential: None,
        }
    }

    /// Create a TURN server configuration with authentication
    pub fn turn(url: String, username: String, credential: String) -> Self {
        Self {
            urls: vec![url],
            username: Some(username),
            credential: Some(credential),
        }
    }

    /// Create from multiple URLs (for failover)
    pub fn from_urls(urls: Vec<String>) -> Self {
        Self {
            urls,
            username: None,
            credential: None,
        }
    }

    /// Add authentication to existing server config
    pub fn with_auth(mut self, username: String, credential: String) -> Self {
        self.username = Some(username);
        self.credential = Some(credential);
        self
    }

    /// Get default Google STUN servers
    pub fn default_stun_servers() -> Vec<Self> {
        vec![
            Self::stun("stun:stun.l.google.com:19302".to_string()),
            Self::stun("stun:stun1.l.google.com:19302".to_string()),
            Self::stun("stun:stun2.l.google.com:19302".to_string()),
            Self::stun("stun:stun3.l.google.com:19302".to_string()),
            Self::stun("stun:stun4.l.google.com:19302".to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stun_server() {
        let server = IceServer::stun("stun:stun.l.google.com:19302".to_string());
        assert_eq!(server.urls.len(), 1);
        assert_eq!(server.urls[0], "stun:stun.l.google.com:19302");
        assert!(server.username.is_none());
        assert!(server.credential.is_none());
    }

    #[test]
    fn test_turn_server() {
        let server = IceServer::turn(
            "turn:turn.example.com:3478".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        assert_eq!(server.urls.len(), 1);
        assert_eq!(server.username, Some("user".to_string()));
        assert_eq!(server.credential, Some("pass".to_string()));
    }

    #[test]
    fn test_with_auth() {
        let server = IceServer::stun("turn:turn.example.com:3478".to_string())
            .with_auth("user".to_string(), "pass".to_string());
        assert_eq!(server.username, Some("user".to_string()));
        assert_eq!(server.credential, Some("pass".to_string()));
    }

    #[test]
    fn test_default_stun_servers() {
        let servers = IceServer::default_stun_servers();
        assert_eq!(servers.len(), 5);
        assert!(servers.iter().all(|s| s.username.is_none()));
    }

    #[test]
    fn test_from_urls() {
        let urls = vec![
            "stun:stun1.example.com:3478".to_string(),
            "stun:stun2.example.com:3478".to_string(),
        ];
        let server = IceServer::from_urls(urls.clone());
        assert_eq!(server.urls, urls);
    }

    #[test]
    fn test_serialization() {
        let server = IceServer::turn(
            "turn:turn.example.com:3478".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        let json = serde_json::to_string(&server).unwrap();
        let deserialized: IceServer = serde_json::from_str(&json).unwrap();
        assert_eq!(server, deserialized);
    }
}
