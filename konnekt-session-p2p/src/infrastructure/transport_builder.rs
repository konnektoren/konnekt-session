use crate::domain::{IceServer, SessionId};
use crate::infrastructure::connection::MatchboxConnection;
use crate::infrastructure::error::Result;
use crate::infrastructure::transport::P2PTransport;
use uuid::Uuid;

/// Builder for creating P2P transports
pub struct P2PTransportBuilder {
    cache_size: usize,
}

impl P2PTransportBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self { cache_size: 100 }
    }

    /// Set message cache size (for resend requests)
    pub fn cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }

    /// Build transport as HOST (creates new session)
    ///
    /// Returns: (transport, session_id, lobby_id)
    pub async fn build_host(
        self,
        signalling_server: &str,
        ice_servers: Vec<IceServer>,
    ) -> Result<(P2PTransport, SessionId, Uuid)> {
        let session_id = SessionId::new();
        let lobby_id = session_id.inner(); // 1:1 mapping

        let room_url = format!("{}/{}", signalling_server, session_id.as_str());

        tracing::info!("🎯 Creating HOST transport for session {}", session_id);
        tracing::info!("📋 Lobby ID: {}", lobby_id);

        let connection = MatchboxConnection::connect(&room_url, ice_servers).await?;
        let transport = P2PTransport::new_host(connection, self.cache_size);

        Ok((transport, session_id, lobby_id))
    }

    /// Build transport as GUEST (joins existing session)
    ///
    /// Returns: (transport, lobby_id)
    pub async fn build_guest(
        self,
        signalling_server: &str,
        session_id: SessionId,
        ice_servers: Vec<IceServer>,
    ) -> Result<(P2PTransport, Uuid)> {
        let lobby_id = session_id.inner(); // 1:1 mapping

        let room_url = format!("{}/{}", signalling_server, session_id.as_str());

        tracing::info!("🎯 Creating GUEST transport for session {}", session_id);
        tracing::info!("📋 Lobby ID: {}", lobby_id);

        let connection = MatchboxConnection::connect(&room_url, ice_servers).await?;
        let transport = P2PTransport::new_guest(connection, self.cache_size);

        Ok((transport, lobby_id))
    }
}

impl Default for P2PTransportBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = P2PTransportBuilder::new();
        assert_eq!(builder.cache_size, 100);
    }

    #[test]
    fn test_builder_custom_cache() {
        let builder = P2PTransportBuilder::new().cache_size(200);
        assert_eq!(builder.cache_size, 200);
    }
}
