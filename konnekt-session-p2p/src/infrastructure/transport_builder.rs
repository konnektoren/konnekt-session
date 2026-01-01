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
    pub fn new() -> Self {
        Self { cache_size: 100 }
    }

    pub fn cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }

    /// Build transport as HOST
    pub async fn build_host(
        self,
        signalling_server: &str,
        ice_servers: Vec<IceServer>,
    ) -> Result<(P2PTransport<MatchboxConnection>, SessionId, Uuid)> {
        let session_id = SessionId::new();
        let lobby_id = session_id.inner();

        let room_url = format!("{}/{}", signalling_server, session_id.as_str());

        tracing::info!("🎯 Creating HOST transport for session {}", session_id);

        let connection = MatchboxConnection::connect(&room_url, ice_servers).await?;
        let transport = P2PTransport::new_host(connection, self.cache_size);

        Ok((transport, session_id, lobby_id))
    }

    /// Build transport as GUEST
    pub async fn build_guest(
        self,
        signalling_server: &str,
        session_id: SessionId,
        ice_servers: Vec<IceServer>,
    ) -> Result<(P2PTransport<MatchboxConnection>, Uuid)> {
        let lobby_id = session_id.inner();

        let room_url = format!("{}/{}", signalling_server, session_id.as_str());

        tracing::info!("🎯 Creating GUEST transport for session {}", session_id);

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
