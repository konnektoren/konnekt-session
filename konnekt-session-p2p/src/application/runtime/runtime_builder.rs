use crate::application::runtime::P2PLoop;
use crate::domain::{IceServer, SessionId};
use crate::infrastructure::{connection::MatchboxConnection, error::Result};
use uuid::Uuid;

/// Builder for creating a P2PLoop with automatic sync
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
}
