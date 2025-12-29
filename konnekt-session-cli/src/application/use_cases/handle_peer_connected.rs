use crate::domain::SessionState;
use crate::infrastructure::error::{CliError, Result};
use konnekt_session_p2p::{P2PSession, PeerId};

pub async fn handle_peer_connected(
    session: &mut P2PSession,
    state: &mut SessionState,
    peer_id: PeerId,
) -> Result<()> {
    tracing::info!("🟢 Peer connected: {}", peer_id);

    // Send our participant info to the new peer
    let participant = state.participant();
    let intro_msg = serde_json::json!({
        "type": "participant_info",
        "participant_id": participant.id().to_string(),
        "name": participant.name(),
        "role": format!("{}", participant.lobby_role()),
        "mode": format!("{}", participant.participation_mode()),
        "joined_at": participant.joined_at().as_millis(),
    });

    let data =
        serde_json::to_vec(&intro_msg).map_err(|e| CliError::Serialization(e.to_string()))?;

    session
        .send_to(peer_id, data)
        .map_err(|e| CliError::MessageSend(e.to_string()))?;

    tracing::info!("📤 Sent participant info to peer {}", peer_id);

    // NEW: Also send full lobby state if we have one
    if let Some(lobby) = state.lobby() {
        tracing::info!("📤 Sending full lobby state to new peer {}", peer_id);

        // Send state for all participants in our lobby
        for p in lobby.participants().values() {
            let participant_state_msg = serde_json::json!({
                "type": "participant_state_sync",
                "participant_id": p.id().to_string(),
                "name": p.name(),
                "role": format!("{}", p.lobby_role()),
                "mode": format!("{}", p.participation_mode()),
                "joined_at": p.joined_at().as_millis(),
            });

            let sync_data = serde_json::to_vec(&participant_state_msg)
                .map_err(|e| CliError::Serialization(e.to_string()))?;

            session
                .send_to(peer_id, sync_data)
                .map_err(|e| CliError::MessageSend(e.to_string()))?;
        }
    }

    tracing::info!("");
    tracing::info!("Connected peers: {}", session.connected_peers().len());

    // Clear host disconnect timer if host reconnected
    if !state.is_host() {
        if let Some(host_peer) = session.find_host_peer() {
            if host_peer == peer_id && state.host_disconnect_elapsed().is_some() {
                tracing::info!("✓ Host reconnected within grace period");
                state.clear_host_disconnect_timer();
            }
        }
    }

    Ok(())
}
