use crate::application::runtime::{P2PLoop, SessionLoop};
use crate::domain::{IceServer, SessionId};
use crate::infrastructure::{connection::MatchboxConnection, error::Result};
use konnekt_session_core::DomainLoop;
use uuid::Uuid;

/// Builder for creating P2P components with automatic sync
pub struct P2PLoopBuilder {
    batch_size: usize,
    queue_size: usize,
}

impl P2PLoopBuilder {
    pub fn new() -> Self {
        Self {
            batch_size: 10,
            queue_size: 100,
        }
    }

    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    pub fn queue_size(mut self, size: usize) -> Self {
        self.queue_size = size;
        self
    }

    /// Build P2P loop for host (creates new session)
    /// Returns (p2p_loop, session_id, lobby_id)
    pub async fn build_host(
        self,
        signalling_server: &str,
        ice_servers: Vec<IceServer>,
    ) -> Result<(P2PLoop, SessionId, Uuid)> {
        let session_id = SessionId::new();
        let lobby_id = session_id.inner(); // 1:1 mapping

        let room_url = format!("{}/{}", signalling_server, session_id.as_str());

        tracing::info!("🎯 Creating HOST session {}", session_id);
        tracing::info!("📋 Lobby ID: {}", lobby_id);

        let connection = MatchboxConnection::connect(&room_url, ice_servers).await?;

        let p2p_loop = P2PLoop::new_host(connection, lobby_id, self.batch_size, self.queue_size);

        Ok((p2p_loop, session_id, lobby_id))
    }

    /// Build P2P loop for guest (joins existing session)
    /// Returns (p2p_loop, lobby_id)
    pub async fn build_guest(
        self,
        signalling_server: &str,
        session_id: SessionId,
        ice_servers: Vec<IceServer>,
    ) -> Result<(P2PLoop, Uuid)> {
        let lobby_id = session_id.inner(); // 1:1 mapping

        let room_url = format!("{}/{}", signalling_server, session_id.as_str());

        tracing::info!("🎯 Joining GUEST session {}", session_id);
        tracing::info!("📋 Lobby ID: {}", lobby_id);

        let connection = MatchboxConnection::connect(&room_url, ice_servers).await?;

        let p2p_loop = P2PLoop::new_guest(connection, lobby_id, self.batch_size, self.queue_size);

        Ok((p2p_loop, lobby_id))
    }

    /// Build complete SessionLoop for HOST (P2P + Core integrated)
    ///
    /// This creates:
    /// - P2P networking layer
    /// - Core domain layer
    /// - Lobby with host participant
    /// - Automatic wiring between layers
    ///
    /// Returns (session_loop, session_id)
    pub async fn build_session_host(
        self,
        signalling_server: &str,
        ice_servers: Vec<IceServer>,
        lobby_name: String,
        host_name: String,
    ) -> Result<(SessionLoop, SessionId)> {
        // 🔧 FIX: Extract values BEFORE consuming self
        let batch_size = self.batch_size;
        let queue_size = self.queue_size;

        // Create P2P layer (consumes self)
        let (p2p_loop, session_id, lobby_id) =
            self.build_host(signalling_server, ice_servers).await?;

        // Create domain layer (using extracted values)
        let mut domain_loop = DomainLoop::new(batch_size, queue_size);

        // Create lobby in domain
        let create_cmd = konnekt_session_core::DomainCommand::CreateLobby {
            lobby_id: Some(lobby_id), // Use same ID as session
            lobby_name,
            host_name,
        };

        domain_loop
            .submit(create_cmd)
            .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))?;

        // Process command to create lobby
        domain_loop.poll();

        // Verify lobby was created
        let events = domain_loop.drain_events();
        if !events
            .iter()
            .any(|e| matches!(e, konnekt_session_core::DomainEvent::LobbyCreated { .. }))
        {
            return Err(crate::infrastructure::error::P2PError::ConnectionFailed(
                "Failed to create lobby".to_string(),
            ));
        }

        // Create unified session loop
        let session_loop = SessionLoop::new_host(p2p_loop, domain_loop, lobby_id);

        tracing::info!("✅ SessionLoop created for HOST");

        Ok((session_loop, session_id))
    }

    /// Build complete SessionLoop for GUEST (P2P + Core integrated)
    ///
    /// This creates:
    /// - P2P networking layer
    /// - Core domain layer (empty, will sync from host)
    /// - Automatic wiring between layers
    ///
    /// Guest will receive full lobby state from host via P2P sync.
    ///
    /// Returns (session_loop, lobby_id)
    pub async fn build_session_guest(
        self,
        signalling_server: &str,
        session_id: SessionId,
        ice_servers: Vec<IceServer>,
    ) -> Result<(SessionLoop, Uuid)> {
        // 🔧 FIX: Extract values BEFORE consuming self
        let batch_size = self.batch_size;
        let queue_size = self.queue_size;

        // Create P2P layer (consumes self)
        let (p2p_loop, lobby_id) = self
            .build_guest(signalling_server, session_id, ice_servers)
            .await?;

        // Create domain layer (using extracted values)
        let domain_loop = DomainLoop::new(batch_size, queue_size);

        // Create unified session loop
        let session_loop = SessionLoop::new_guest(p2p_loop, domain_loop, lobby_id);

        tracing::info!("✅ SessionLoop created for GUEST");

        Ok((session_loop, lobby_id))
    }
}

impl Default for P2PLoopBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = P2PLoopBuilder::new();
        assert_eq!(builder.batch_size, 10);
        assert_eq!(builder.queue_size, 100);
    }

    #[test]
    fn test_builder_custom() {
        let builder = P2PLoopBuilder::new().batch_size(20).queue_size(200);
        assert_eq!(builder.batch_size, 20);
        assert_eq!(builder.queue_size, 200);
    }

    // Integration tests with real connections would go in tests/ directory
}
