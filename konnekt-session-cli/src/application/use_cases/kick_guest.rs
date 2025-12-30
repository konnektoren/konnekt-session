use crate::domain::SessionState;
use crate::infrastructure::error::{CliError, Result};
use konnekt_session_p2p::{DomainEvent, P2PSession};
use uuid::Uuid;

/// Kick a guest from the lobby (host only)
pub async fn kick_guest(
    session: &mut P2PSession,
    state: &mut SessionState,
    guest_id: Uuid,
) -> Result<()> {
    // Only host can kick
    if !state.is_host() {
        return Err(CliError::InvalidConfig(
            "Only host can kick guests".to_string(),
        ));
    }

    let host_id = state.participant().id();

    // Execute the command on the domain model
    let lobby = state
        .lobby_mut()
        .ok_or_else(|| CliError::InvalidConfig("No lobby".to_string()))?;

    let kicked_participant = lobby
        .kick_guest(guest_id, host_id)
        .map_err(|e| CliError::InvalidConfig(e.to_string()))?;

    tracing::info!(
        "✅ Kicked guest: {} (ID: {})",
        kicked_participant.name(),
        guest_id
    );

    // Create and broadcast the event
    let event = DomainEvent::GuestKicked {
        participant_id: guest_id,
        kicked_by: host_id,
    };

    tracing::info!("📤 Broadcasting GuestKicked event...");

    session.create_event(event).map_err(|e| {
        tracing::error!("❌ Failed to broadcast kick event: {:?}", e);
        CliError::MessageSend(e.to_string())
    })?;

    tracing::info!("✅ GuestKicked event broadcast successful");

    Ok(())
}
