use crate::domain::SessionState;
use crate::infrastructure::error::Result;
use konnekt_session_p2p::{DomainEvent, P2PSession, PeerId};

pub async fn handle_peer_disconnected(
    session: &mut P2PSession,
    state: &mut SessionState,
    peer_id: PeerId,
) -> Result<()> {
    tracing::warn!("🔴 Peer disconnected: {} (grace period started)", peer_id);

    // If we're the host, broadcast a GuestLeft event immediately
    if state.is_host() {
        // Try to find the participant ID for this peer
        if let Some(participant_id) = session.find_participant_by_peer(&peer_id) {
            tracing::info!(
                "Broadcasting GuestLeft event for participant {}",
                participant_id
            );

            // Remove from our local state first
            if let Some(lobby) = state.lobby_mut() {
                lobby.participants_mut().remove(&participant_id);
            }

            // Then broadcast to other guests
            let event = DomainEvent::GuestLeft { participant_id };

            if let Err(e) = session.create_event(event) {
                tracing::warn!("Failed to broadcast GuestLeft event: {:?}", e);
            }
        }
    } else {
        // If we're a guest and detect peer disconnect, just log it
        // The host will broadcast the GuestLeft event
        tracing::debug!(
            "Guest detected peer {} disconnected, waiting for host event",
            peer_id
        );
    }

    Ok(())
}
