use crate::application::{ConnectionEvent, SessionConfig};
use crate::domain::{PeerId, SessionId};
use crate::infrastructure::{connection::MatchboxConnection, error::Result};

/// Application service: High-level P2P session management
pub struct P2PSession {
    session_id: SessionId,
    connection: MatchboxConnection,
}

impl P2PSession {
    /// Create a new P2P session (as host) with default config
    pub async fn create_host(signalling_server: &str) -> Result<Self> {
        let config = SessionConfig::new(signalling_server.to_string());
        Self::create_host_with_config(config).await
    }

    /// Create a new P2P session (as host) with custom config
    pub async fn create_host_with_config(config: SessionConfig) -> Result<Self> {
        let session_id = SessionId::new();
        Self::join_with_config(config, session_id).await
    }

    /// Join an existing P2P session (as guest) with default config
    pub async fn join(signalling_server: &str, session_id: SessionId) -> Result<Self> {
        let config = SessionConfig::new(signalling_server.to_string());
        Self::join_with_config(config, session_id).await
    }

    /// Join an existing P2P session (as guest) with custom config
    pub async fn join_with_config(config: SessionConfig, session_id: SessionId) -> Result<Self> {
        let room_url = format!("{}/{}", config.signalling_server, session_id.as_str());

        tracing::info!("Joining session {} at {}", session_id, room_url);
        tracing::debug!("Using {} ICE servers", config.ice_servers.len());

        let connection = MatchboxConnection::connect(&room_url, config.ice_servers).await?;

        Ok(P2PSession {
            session_id,
            connection,
        })
    }

    /// Get the session ID
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// Get our local peer ID
    pub fn local_peer_id(&self) -> Option<PeerId> {
        self.connection.local_peer_id()
    }

    /// Get list of connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.connection.connected_peers()
    }

    /// Send data to a specific peer
    pub fn send_to(&mut self, peer: PeerId, data: Vec<u8>) -> Result<()> {
        self.connection.send_to(peer, data)
    }

    /// Broadcast data to all peers
    pub fn broadcast(&mut self, data: Vec<u8>) -> Result<()> {
        self.connection.broadcast(data)
    }

    /// Poll for connection events
    pub fn poll_events(&mut self) -> Vec<ConnectionEvent> {
        self.connection.poll_events()
    }
}
