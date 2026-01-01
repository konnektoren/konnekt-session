use crate::domain::{IceServer, SessionId};
use crate::infrastructure::error::Result;
use crate::infrastructure::transport_builder::P2PTransportBuilder;
use konnekt_session_core::DomainLoop;
use uuid::Uuid;

use super::session_loop_v2::MatchboxSessionLoop;

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
    pub async fn build_host(
        self,
        signalling_server: &str,
        ice_servers: Vec<IceServer>,
        lobby_name: String,
        host_name: String,
    ) -> Result<(MatchboxSessionLoop, SessionId)> {
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
            lobby_id: Some(lobby_id),
            lobby_name,
            host_name,
        };

        domain
            .submit(create_cmd)
            .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))?;

        domain.poll();

        let events = domain.drain_events();
        if !events
            .iter()
            .any(|e| matches!(e, konnekt_session_core::DomainEvent::LobbyCreated { .. }))
        {
            return Err(crate::infrastructure::error::P2PError::ConnectionFailed(
                "Failed to create lobby".to_string(),
            ));
        }

        let session_loop = MatchboxSessionLoop::new(domain, transport, true, lobby_id);

        tracing::info!("✅ SessionLoopV2 created as HOST");

        Ok((session_loop, session_id))
    }

    /// Build complete SessionLoopV2 for GUEST
    pub async fn build_guest(
        self,
        signalling_server: &str,
        session_id: SessionId,
        ice_servers: Vec<IceServer>,
    ) -> Result<(MatchboxSessionLoop, Uuid)> {
        tracing::info!("🎯 Building SessionLoopV2 as GUEST");

        let (transport, lobby_id) = P2PTransportBuilder::new()
            .cache_size(self.cache_size)
            .build_guest(signalling_server, session_id, ice_servers)
            .await?;

        let domain = DomainLoop::new(self.batch_size, self.queue_size);

        let session_loop = MatchboxSessionLoop::new(domain, transport, false, lobby_id);

        tracing::info!("✅ SessionLoopV2 created as GUEST");

        Ok((session_loop, lobby_id))
    }
}

impl Default for SessionLoopV2Builder {
    fn default() -> Self {
        Self::new()
    }
}
