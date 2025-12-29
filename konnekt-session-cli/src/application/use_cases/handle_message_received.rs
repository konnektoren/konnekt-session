use crate::domain::SessionState;
use crate::infrastructure::error::{CliError, Result};
use konnekt_session_core::{Lobby, Participant, ParticipationMode};
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
                    "participant_state_sync" => {
                        // NEW: Handle full state sync
                        handle_participant_state_sync(state, &msg)?;
                    }
                    "host_delegated" => {
                        handle_host_delegation(state, &msg)?;
                    }
                    "participation_mode_changed" => {
                        handle_participation_mode_changed(state, &msg)?;
                    }
                    _ => {
                        tracing::debug!("📥 Received message from {}: {:?}", from, msg);
                    }
                }
            }
        }
        Err(_) => {
            tracing::debug!("📥 Received {} bytes from {}", data.len(), from);
        }
    }

    Ok(())
}

fn handle_participant_state_sync(state: &mut SessionState, msg: &serde_json::Value) -> Result<()> {
    let participant_id = msg
        .get("participant_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            CliError::InvalidConfig("Invalid participant_id in participant_state_sync".to_string())
        })?;

    let name = msg.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
        CliError::InvalidConfig("Missing name in participant_state_sync".to_string())
    })?;

    let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("Guest");

    let mode_str = msg.get("mode").and_then(|v| v.as_str()).unwrap_or("Active");

    let participation_mode = match mode_str {
        "Active" => ParticipationMode::Active,
        "Spectating" => ParticipationMode::Spectating,
        _ => ParticipationMode::Active,
    };

    tracing::info!("📥 Received state sync for participant '{}'", name);
    tracing::info!("   ID: {}", participant_id);
    tracing::info!("   Role: {}", role);
    tracing::info!("   Mode: {}", mode_str);

    // Get our local participant ID first (before any borrows)
    let local_participant_id = state.participant().id();

    // Update our lobby state if we have one
    if let Some(lobby) = state.lobby_mut() {
        // Check if this participant exists
        if let Some(participant) = lobby.participants_mut().get_mut(&participant_id) {
            // Update existing participant
            participant.force_participation_mode(participation_mode);
            tracing::info!(
                "   ✓ Updated existing participant '{}' mode to {}",
                name,
                mode_str
            );
        } else if participant_id != local_participant_id {
            // This is a new participant we haven't seen yet
            let is_host = role == "Host";

            if is_host {
                tracing::warn!("   ⚠️  Received sync for unknown host (ignoring)");
            } else {
                // Create new guest with correct mode
                let mut guest = Participant::new_guest(name.to_string())
                    .map_err(|e| CliError::ParticipantCreation(e.to_string()))?;
                guest.force_participation_mode(participation_mode);

                lobby
                    .add_guest(guest)
                    .map_err(|e| CliError::InvalidConfig(e.to_string()))?;

                tracing::info!(
                    "   ✓ Added new participant '{}' with mode {}",
                    name,
                    mode_str
                );
            }
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

    let mode_str = msg.get("mode").and_then(|v| v.as_str()).unwrap_or("Active");

    let participation_mode = match mode_str {
        "Active" => ParticipationMode::Active,
        "Spectating" => ParticipationMode::Spectating,
        _ => ParticipationMode::Active,
    };

    let participant_id = msg
        .get("participant_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            CliError::InvalidConfig("Invalid participant_id in participant_info".to_string())
        })?;

    let is_host = role == "Host";

    tracing::info!("📥 Received participant info from peer {}:", peer_id);
    tracing::info!("   Participant ID: {}", participant_id);
    tracing::info!("   Name: {}", name);
    tracing::info!("   Role: {}", role);
    tracing::info!("   Mode: {}", mode_str);

    // Register in P2P session's peer registry
    session.register_peer_participant(peer_id, participant_id, name.to_string(), is_host);

    // Update lobby state
    if state.is_host() {
        // We are the host - add this guest to our lobby
        if !is_host {
            if let Some(lobby) = state.lobby_mut() {
                if !lobby.participants().contains_key(&participant_id) {
                    let mut guest = Participant::new_guest(name.to_string())
                        .map_err(|e| CliError::ParticipantCreation(e.to_string()))?;

                    guest.force_participation_mode(participation_mode);

                    lobby
                        .add_guest(guest)
                        .map_err(|e| CliError::InvalidConfig(e.to_string()))?;

                    tracing::info!("   ✓ Added guest '{}' to lobby (mode: {})", name, mode_str);
                } else {
                    if let Some(participant) = lobby.participants_mut().get_mut(&participant_id) {
                        participant.force_participation_mode(participation_mode);
                        tracing::info!("   ✓ Updated guest '{}' mode to {}", name, mode_str);
                    }
                }
            }
        }
    } else {
        // We are a guest
        if is_host {
            if state.lobby().is_none() {
                let mut host = Participant::new_host(name.to_string())
                    .map_err(|e| CliError::ParticipantCreation(e.to_string()))?;

                host.force_participation_mode(participation_mode);

                let mut lobby = Lobby::new("CLI Lobby".to_string(), host)
                    .map_err(|e| CliError::InvalidConfig(e.to_string()))?;

                lobby
                    .add_guest(state.participant().clone())
                    .map_err(|e| CliError::InvalidConfig(e.to_string()))?;

                state.set_lobby(lobby);
                tracing::info!("   ✓ Joined lobby with host '{}'", name);
            }
        } else {
            if let Some(lobby) = state.lobby_mut() {
                if !lobby.participants().contains_key(&participant_id) {
                    let mut guest = Participant::new_guest(name.to_string())
                        .map_err(|e| CliError::ParticipantCreation(e.to_string()))?;

                    guest.force_participation_mode(participation_mode);

                    lobby
                        .add_guest(guest)
                        .map_err(|e| CliError::InvalidConfig(e.to_string()))?;

                    tracing::info!("   ✓ Added guest '{}' to lobby (mode: {})", name, mode_str);
                } else {
                    if let Some(participant) = lobby.participants_mut().get_mut(&participant_id) {
                        participant.force_participation_mode(participation_mode);
                        tracing::info!("   ✓ Updated guest '{}' mode to {}", name, mode_str);
                    }
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

fn handle_participation_mode_changed(
    state: &mut SessionState,
    msg: &serde_json::Value,
) -> Result<()> {
    let participant_id = msg
        .get("participant_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            CliError::InvalidConfig(
                "Invalid participant_id in participation_mode_changed".to_string(),
            )
        })?;

    let new_mode_str = msg
        .get("new_mode")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            CliError::InvalidConfig("Missing new_mode in participation_mode_changed".to_string())
        })?;

    let new_mode = match new_mode_str {
        "Active" => ParticipationMode::Active,
        "Spectating" => ParticipationMode::Spectating,
        _ => {
            return Err(CliError::InvalidConfig(format!(
                "Invalid participation mode: {}",
                new_mode_str
            )));
        }
    };

    let forced = msg.get("forced").and_then(|v| v.as_bool()).unwrap_or(false);

    tracing::info!("📥 Participation mode changed");
    tracing::info!("   Participant: {}", participant_id);
    tracing::info!("   New mode: {}", new_mode);
    tracing::info!("   Forced: {}", forced);

    // Update lobby state
    if let Some(lobby) = state.lobby_mut() {
        if let Some(participant) = lobby.participants_mut().get_mut(&participant_id) {
            let old_mode = participant.participation_mode();
            participant.force_participation_mode(new_mode);
            tracing::info!(
                "✓ Updated participant '{}' mode: {} → {}",
                participant.name(),
                old_mode,
                new_mode
            );
        } else {
            tracing::warn!("⚠️  Participant {} not found in lobby", participant_id);
        }
    } else {
        tracing::warn!("⚠️  No lobby found to update participant mode");
    }

    Ok(())
}
