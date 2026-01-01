use crate::domain::{IceServer, SessionId};
use crate::infrastructure::error::Result;
use crate::infrastructure::transport_builder::P2PTransportBuilder;
use konnekt_session_core::DomainLoop;
use uuid::Uuid;

use super::session_loop_v2::SessionLoopV2;

/// Builder for creating complete SessionLoopV2 (P2P + Domain integrated)
pub struct SessionLoopV2Builder {
    batch_size: usize,
    queue_size: usize,
    cache_size: usize,
}

impl SessionLoopV2Builder {
    pub fn new() -> Self {
        Self {
            batch_size: 10,
            queue_size: 100,
            cache_size: 100,
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

    pub fn cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }

    /// Build complete SessionLoopV2 for HOST
    ///
    /// This creates:
    /// - P2P transport layer
    /// - Core domain layer
    /// - Lobby with host participant
    /// - Automatic wiring between layers
    ///
    /// Returns: (session_loop, session_id)
    pub async fn build_host(
        self,
        signalling_server: &str,
        ice_servers: Vec<IceServer>,
        lobby_name: String,
        host_name: String,
    ) -> Result<(SessionLoopV2, SessionId)> {
        tracing::info!("🎯 Building SessionLoopV2 as HOST");

        // 1. Create P2P transport
        let (transport, session_id, lobby_id) = P2PTransportBuilder::new()
            .cache_size(self.cache_size)
            .build_host(signalling_server, ice_servers)
            .await?;

        // 2. Create domain layer
        let mut domain = DomainLoop::new(self.batch_size, self.queue_size);

        // 3. Create lobby in domain
        let create_cmd = konnekt_session_core::DomainCommand::CreateLobby {
            lobby_id: Some(lobby_id), // Use same ID as session
            lobby_name,
            host_name,
        };

        domain
            .submit(create_cmd)
            .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))?;

        // Process command to create lobby
        domain.poll();

        // Verify lobby was created
        let events = domain.drain_events();
        if !events
            .iter()
            .any(|e| matches!(e, konnekt_session_core::DomainEvent::LobbyCreated { .. }))
        {
            return Err(crate::infrastructure::error::P2PError::ConnectionFailed(
                "Failed to create lobby".to_string(),
            ));
        }

        // 4. Create unified session loop
        let session_loop = SessionLoopV2::new(domain, transport, true, lobby_id);

        tracing::info!("✅ SessionLoopV2 created as HOST");

        Ok((session_loop, session_id))
    }

    /// Build complete SessionLoopV2 for GUEST
    ///
    /// This creates:
    /// - P2P transport layer
    /// - Core domain layer (empty, will sync from host)
    /// - Automatic wiring between layers
    ///
    /// Guest will receive full lobby state from host via P2P sync.
    ///
    /// Returns: (session_loop, lobby_id)
    pub async fn build_guest(
        self,
        signalling_server: &str,
        session_id: SessionId,
        ice_servers: Vec<IceServer>,
    ) -> Result<(SessionLoopV2, Uuid)> {
        tracing::info!("🎯 Building SessionLoopV2 as GUEST");

        // 1. Create P2P transport
        let (transport, lobby_id) = P2PTransportBuilder::new()
            .cache_size(self.cache_size)
            .build_guest(signalling_server, session_id, ice_servers)
            .await?;

        // 2. Create domain layer (empty)
        let domain = DomainLoop::new(self.batch_size, self.queue_size);

        // 3. Create unified session loop
        let session_loop = SessionLoopV2::new(domain, transport, false, lobby_id);

        tracing::info!("✅ SessionLoopV2 created as GUEST");

        Ok((session_loop, lobby_id))
    }
}

impl Default for SessionLoopV2Builder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = SessionLoopV2Builder::new();
        assert_eq!(builder.batch_size, 10);
        assert_eq!(builder.queue_size, 100);
        assert_eq!(builder.cache_size, 100);
    }

    #[test]
    fn test_builder_custom() {
        let builder = SessionLoopV2Builder::new()
            .batch_size(20)
            .queue_size(200)
            .cache_size(50);

        assert_eq!(builder.batch_size, 20);
        assert_eq!(builder.queue_size, 200);
        assert_eq!(builder.cache_size, 50);
    }
}
