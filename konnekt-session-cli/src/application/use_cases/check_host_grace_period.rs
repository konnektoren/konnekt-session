use crate::domain::SessionState;
use crate::infrastructure::error::{CliError, Result};
use konnekt_session_p2p::P2PSession;
use std::time::Duration;

const HOST_GRACE_PERIOD: Duration = Duration::from_secs(30);

pub async fn check_host_grace_period(
    session: &mut P2PSession,
    state: &mut SessionState,
) -> Result<()> {
    // Only guests need to monitor host status
    if state.is_host() {
        return Ok(());
    }

    // Check if we have a lobby
    if state.lobby().is_none() {
        return Ok(());
    }

    // Check if grace period has expired
    if let Some(elapsed) = state.host_disconnect_elapsed() {
        if elapsed >= HOST_GRACE_PERIOD {
            tracing::warn!("⏰ Host grace period expired! Initiating host delegation...");

            let lobby = state
                .lobby_mut()
                .ok_or_else(|| CliError::InvalidConfig("No lobby".to_string()))?;

            match lobby.auto_delegate_host() {
                Ok(new_host_id) => {
                    if new_host_id == state.participant().id() {
                        // We became the host!
                        tracing::info!("👑 You are now the HOST!");
                        state.promote_to_host();

                        // Broadcast the delegation event to all peers
                        let delegation_msg = serde_json::json!({
                            "type": "host_delegated",
                            "new_host_id": new_host_id.to_string(),
                            "reason": "timeout"
                        });

                        let data = serde_json::to_vec(&delegation_msg)
                            .map_err(|e| CliError::Serialization(e.to_string()))?;

                        session
                            .broadcast(data)
                            .map_err(|e| CliError::MessageSend(e.to_string()))?;

                        tracing::info!("📤 Broadcast host delegation event");
                    } else {
                        tracing::info!("ℹ️  Guest '{}' became the new host", new_host_id);
                    }

                    // Clear the disconnect timer
                    state.clear_host_disconnect_timer();
                }
                Err(e) => {
                    tracing::warn!("❌ Failed to delegate host: {:?}", e);
                    state.clear_host_disconnect_timer();
                }
            }
        }
    }

    Ok(())
}
