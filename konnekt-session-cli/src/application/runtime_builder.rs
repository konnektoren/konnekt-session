use crate::application::runtime::DualLoopRuntime;
use crate::infrastructure::Result; // ✅ Uses CliError::Result
use konnekt_session_core::DomainLoop;
use konnekt_session_p2p::{IceServer, P2PLoopBuilder, SessionId};

/// Builder for creating a DualLoopRuntime
pub struct RuntimeBuilder {
    domain_batch_size: usize,
    domain_queue_size: usize,
    p2p_batch_size: usize,
    p2p_queue_size: usize,
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self {
            domain_batch_size: 10,
            domain_queue_size: 100,
            p2p_batch_size: 10,
            p2p_queue_size: 100,
        }
    }

    pub fn domain_batch_size(mut self, size: usize) -> Self {
        self.domain_batch_size = size;
        self
    }

    pub fn domain_queue_size(mut self, size: usize) -> Self {
        self.domain_queue_size = size;
        self
    }

    pub fn p2p_batch_size(mut self, size: usize) -> Self {
        self.p2p_batch_size = size;
        self
    }

    pub fn p2p_queue_size(mut self, size: usize) -> Self {
        self.p2p_queue_size = size;
        self
    }

    /// Build runtime for host (creates new session)
    /// Returns (runtime, session_id, lobby_id)
    pub async fn build_host(
        self,
        signalling_server: &str,
        ice_servers: Vec<IceServer>,
    ) -> Result<(DualLoopRuntime, SessionId, uuid::Uuid)> {
        let domain_loop = DomainLoop::new(self.domain_batch_size, self.domain_queue_size);

        // Build P2P loop with automatic sync
        let (p2p_loop, session_id, lobby_id) = P2PLoopBuilder::new()
            .batch_size(self.p2p_batch_size)
            .queue_size(self.p2p_queue_size)
            .build_host(signalling_server, ice_servers)
            .await?;

        let runtime = DualLoopRuntime::new(domain_loop, p2p_loop);

        Ok((runtime, session_id, lobby_id))
    }

    /// Build runtime for guest (joins existing session)
    /// Returns (runtime, session_id, lobby_id)
    pub async fn build_guest(
        self,
        signalling_server: &str,
        session_id: SessionId,
        ice_servers: Vec<IceServer>,
    ) -> Result<(DualLoopRuntime, SessionId, uuid::Uuid)> {
        let domain_loop = DomainLoop::new(self.domain_batch_size, self.domain_queue_size);

        // Build P2P loop with automatic sync
        let (p2p_loop, lobby_id) = P2PLoopBuilder::new()
            .batch_size(self.p2p_batch_size)
            .queue_size(self.p2p_queue_size)
            .build_guest(signalling_server, session_id.clone(), ice_servers)
            .await?;

        let runtime = DualLoopRuntime::new(domain_loop, p2p_loop);

        Ok((runtime, session_id, lobby_id))
    }
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = RuntimeBuilder::new();
        assert_eq!(builder.domain_batch_size, 10);
        assert_eq!(builder.domain_queue_size, 100);
        assert_eq!(builder.p2p_batch_size, 10);
        assert_eq!(builder.p2p_queue_size, 100);
    }

    #[test]
    fn test_builder_custom() {
        let builder = RuntimeBuilder::new()
            .domain_batch_size(20)
            .domain_queue_size(200)
            .p2p_batch_size(30)
            .p2p_queue_size(300);

        assert_eq!(builder.domain_batch_size, 20);
        assert_eq!(builder.domain_queue_size, 200);
        assert_eq!(builder.p2p_batch_size, 30);
        assert_eq!(builder.p2p_queue_size, 300);
    }
}
