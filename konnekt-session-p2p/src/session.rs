use crate::{Connection, ConnectionEvent, P2PError, PeerId, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Unique identifier for a P2P session (lobby)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);

impl SessionId {
    /// Create a new random session ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse a session ID from a string
    pub fn parse(s: &str) -> Result<Self> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|e| P2PError::InvalidSessionId(e.to_string()))
    }

    /// Get the session ID as a string
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// High-level P2P session management
pub struct P2PSession {
    session_id: SessionId,
    connection: Connection,
    event_rx: mpsc::UnboundedReceiver<ConnectionEvent>,
}

impl P2PSession {
    /// Create a new P2P session (as host)
    pub async fn create_host(signalling_server: &str) -> Result<Self> {
        let session_id = SessionId::new();
        Self::join(signalling_server, session_id).await
    }

    /// Join an existing P2P session (as guest)
    pub async fn join(signalling_server: &str, session_id: SessionId) -> Result<Self> {
        // Build Matchbox room URL
        let room_url = format!("{}/{}", signalling_server, session_id.as_str());

        tracing::info!("Joining session {} at {}", session_id, room_url);

        let connection = Connection::connect(&room_url).await?;
        let event_rx = connection.subscribe();

        Ok(P2PSession {
            session_id,
            connection,
            event_rx,
        })
    }

    /// Get the session ID
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// Get our local peer ID
    pub fn local_peer_id(&self) -> PeerId {
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

    /// Get event receiver
    pub async fn next_event(&mut self) -> Option<ConnectionEvent> {
        self.event_rx.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_new() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_session_id_parse() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let session_id = SessionId::parse(uuid_str).unwrap();

        assert_eq!(session_id.as_str(), uuid_str);
    }

    #[test]
    fn test_session_id_parse_invalid() {
        let result = SessionId::parse("not-a-uuid");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_id_display() {
        let session_id = SessionId::new();
        let display = session_id.to_string();

        // Should be a valid UUID string
        assert!(SessionId::parse(&display).is_ok());
    }

    #[test]
    fn test_session_id_serialization() {
        let session_id = SessionId::new();

        let json = serde_json::to_string(&session_id).unwrap();
        let deserialized: SessionId = serde_json::from_str(&json).unwrap();

        assert_eq!(session_id, deserialized);
    }

    #[test]
    fn test_session_id_default() {
        let id1 = SessionId::default();
        let id2 = SessionId::default();

        // Default should create new UUIDs
        assert_ne!(id1, id2);
    }
}
