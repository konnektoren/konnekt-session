use crate::domain::SessionState;
use crate::infrastructure::error::{CliError, Result};
use konnekt_session_core::{Lobby, Participant};
use konnekt_session_p2p::{P2PSession, PeerId};
use uuid::Uuid;

pub async fn handle_message_received(
    session: &mut P2PSession,
    state: &mut SessionState,
    from: PeerId,
    data: Vec<u8>,
) -> Result<()> {
    // Try to parse as JSON
    match serde_json::from_slice::<serde_json::Value>(&data) {
        Ok(msg) => {
            if let Some(msg_type) = msg.get("type").and_then(|v| v.as_str()) {
                match msg_type {
                    "participant_info" => {
                        handle_participant_info(session, state, from, &msg)?;
                    }
                    "host_delegated" => {
                        handle_host_delegation(state, &msg)?;
                    }
                    _ => {
                        tracing::info!("📥 Received message from {}: {:?}", from, msg);
                    }
                }
            }
        }
        Err(_) => {
            tracing::info!("📥 Received {} bytes from {}", data.len(), from);
        }
    }

    Ok(())
}

fn handle_participant_info(
    session: &mut P2PSession,
    state: &mut SessionState,
    peer_id: PeerId,
    msg: &serde_json::Value,
) -> Result<()> {
    let name = msg
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CliError::InvalidConfig("Missing name in participant_info".to_string()))?;

    let role = msg
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");

    let mode = msg
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");

    let participant_id = msg
        .get("participant_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            CliError::InvalidConfig("Invalid participant_id in participant_info".to_string())
        })?;

    let is_host = role == "Host";

    tracing::info!("📥 Received participant info from peer {}:", peer_id);
    tracing::info!("   Name: {}", name);
    tracing::info!("   Role: {}", role);
    tracing::info!("   Mode: {}", mode);

    // Register in P2P session's peer registry
    session.register_peer_participant(peer_id, participant_id, name.to_string(), is_host);

    // Update lobby state
    if state.is_host() {
        // We are the host - add this guest to our lobby
        if !is_host {
            if let Some(lobby) = state.lobby_mut() {
                // Check if we already have this participant
                if !lobby.participants().contains_key(&participant_id) {
                    let guest = Participant::new_guest(name.to_string())
                        .map_err(|e| CliError::ParticipantCreation(e.to_string()))?;

                    lobby
                        .add_guest(guest)
                        .map_err(|e| CliError::InvalidConfig(e.to_string()))?;

                    tracing::info!("✓ Added guest '{}' to lobby", name);
                }
            }
        }
    } else {
        // We are a guest
        if is_host {
            // This is the host - create/join lobby if we haven't
            if state.lobby().is_none() {
                let host = Participant::new_host(name.to_string())
                    .map_err(|e| CliError::ParticipantCreation(e.to_string()))?;

                let mut lobby = Lobby::new("CLI Lobby".to_string(), host)
                    .map_err(|e| CliError::InvalidConfig(e.to_string()))?;

                // Add ourselves as a guest
                lobby
                    .add_guest(state.participant().clone())
                    .map_err(|e| CliError::InvalidConfig(e.to_string()))?;

                state.set_lobby(lobby);
                tracing::info!("✓ Joined lobby with host '{}'", name);
            }
        } else {
            // This is another guest - add them to our lobby
            if let Some(lobby) = state.lobby_mut() {
                // Check if we already have this participant
                if !lobby.participants().contains_key(&participant_id) {
                    let guest = Participant::new_guest(name.to_string())
                        .map_err(|e| CliError::ParticipantCreation(e.to_string()))?;

                    lobby
                        .add_guest(guest)
                        .map_err(|e| CliError::InvalidConfig(e.to_string()))?;

                    tracing::info!("✓ Added guest '{}' to lobby", name);
                }
            }
        }
    }

    Ok(())
}

fn handle_host_delegation(state: &mut SessionState, msg: &serde_json::Value) -> Result<()> {
    let new_host_id = msg
        .get("new_host_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            CliError::InvalidConfig("Invalid new_host_id in host_delegated".to_string())
        })?;

    let reason = msg
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    tracing::info!("📥 Received host delegation event");
    tracing::info!("   New host ID: {}", new_host_id);
    tracing::info!("   Reason: {}", reason);

    // Clear any pending disconnect timer
    state.clear_host_disconnect_timer();

    // Update lobby state
    if let Some(lobby) = state.lobby_mut() {
        lobby
            .delegate_host(new_host_id)
            .map_err(|e| CliError::InvalidConfig(e.to_string()))?;

        if new_host_id == state.participant().id() {
            tracing::info!("👑 You are now the HOST!");
            state.promote_to_host();
        }
    }

    Ok(())
}
