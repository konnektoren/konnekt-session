use crate::domain::SessionState;
use crate::infrastructure::error::{CliError, Result};
use konnekt_session_core::ParticipationMode;
use konnekt_session_p2p::{DomainEvent, P2PSession};

/// Toggle participation mode for the local user
pub async fn toggle_participation_mode(
    session: &mut P2PSession,
    state: &mut SessionState,
    activity_in_progress: bool,
) -> Result<()> {
    let participant_id = state.participant().id();
    let requester_id = state.participant().id(); // Same as participant for self-toggle

    // Execute the command on the domain model
    let lobby = state
        .lobby_mut()
        .ok_or_else(|| CliError::InvalidConfig("No lobby".to_string()))?;

    let new_mode = lobby
        .toggle_participation_mode(participant_id, requester_id, activity_in_progress)
        .map_err(|e| CliError::InvalidConfig(e.to_string()))?;

    tracing::info!("Toggled to {:?} mode", new_mode);

    // If we're the host, create and broadcast the event
    if state.is_host() {
        let event = DomainEvent::ParticipationModeChanged {
            participant_id,
            new_mode: format!("{}", new_mode),
        };

        session
            .create_event(event)
            .map_err(|e| CliError::MessageSend(e.to_string()))?;

        tracing::info!("📤 Broadcast participation mode change");
    } else {
        // Guest: send request to host (via legacy message for now)
        // TODO: Replace with proper command/event flow
        let request = serde_json::json!({
            "type": "request_mode_change",
            "participant_id": participant_id.to_string(),
            "new_mode": format!("{}", new_mode),
        });

        let data =
            serde_json::to_vec(&request).map_err(|e| CliError::Serialization(e.to_string()))?;

        session
            .broadcast(data)
            .map_err(|e| CliError::MessageSend(e.to_string()))?;

        tracing::info!("📤 Sent mode change request to host");
    }

    Ok(())
}
