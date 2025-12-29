use crate::domain::SessionState;
use crate::infrastructure::error::{CliError, Result};
use konnekt_session_core::ParticipationMode;
use konnekt_session_p2p::P2PSession;

pub async fn toggle_participation_mode(
    session: &mut P2PSession,
    state: &mut SessionState,
) -> Result<ParticipationMode> {
    // Toggle our local participation mode
    let new_mode = state
        .toggle_participation_mode()
        .map_err(|e| CliError::InvalidConfig(e))?;

    tracing::info!("Toggled participation mode to: {}", new_mode);

    // Broadcast the change to all peers
    let participant_id = state.participant().id();
    let mode_change_msg = serde_json::json!({
        "type": "participation_mode_changed",
        "participant_id": participant_id.to_string(),
        "new_mode": format!("{}", new_mode),
        "forced": false,
    });

    let data =
        serde_json::to_vec(&mode_change_msg).map_err(|e| CliError::Serialization(e.to_string()))?;

    session
        .broadcast(data)
        .map_err(|e| CliError::MessageSend(e.to_string()))?;

    tracing::info!("📤 Broadcast participation mode change");

    Ok(new_mode)
}
