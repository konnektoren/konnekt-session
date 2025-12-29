use crate::domain::SessionState;
use crate::infrastructure::error::Result;
use konnekt_session_p2p::{P2PSession, PeerId};

pub async fn handle_peer_disconnected(
    session: &mut P2PSession,
    state: &mut SessionState,
    peer_id: PeerId,
) -> Result<()> {
    tracing::warn!("🔴 Peer disconnected: {} (grace period started)", peer_id);

    // Note: We don't remove from lobby yet - wait for timeout
    // The PeerTimedOut event will handle removal after grace period

    Ok(())
}
