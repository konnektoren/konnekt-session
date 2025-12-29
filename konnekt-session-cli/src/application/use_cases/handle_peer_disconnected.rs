use crate::domain::SessionState;
use crate::infrastructure::error::Result;
use konnekt_session_p2p::{P2PSession, PeerId};

pub async fn handle_peer_disconnected(
    _session: &mut P2PSession,
    state: &mut SessionState,
    peer_id: PeerId,
) -> Result<()> {
    tracing::warn!("🔴 Peer disconnected: {}", peer_id);

    // Check if the disconnected peer was the host
    let was_host = state.is_peer_host(&peer_id);

    // Remove from peer mapping
    state.remove_peer_mapping(&peer_id);

    if was_host && !state.is_host() {
        tracing::warn!("⚠️  Host disconnected! Starting 30-second grace period...");
        state.start_host_disconnect_timer();
    }

    Ok(())
}
