use crate::domain::SessionState;
use crate::infrastructure::error::Result;
use konnekt_session_p2p::{P2PSession, PeerId};

pub async fn handle_peer_disconnected(
    _session: &mut P2PSession,
    _state: &mut SessionState,
    peer_id: PeerId,
) -> Result<()> {
    tracing::warn!("🔴 Peer disconnected: {} (grace period started)", peer_id);
    Ok(())
}
