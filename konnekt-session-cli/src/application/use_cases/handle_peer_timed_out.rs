use crate::domain::SessionState;
use crate::infrastructure::error::{CliError, Result};
use konnekt_session_p2p::{DomainEvent, P2PSession};
use uuid::Uuid;

pub async fn handle_peer_timed_out(
    session: &mut P2PSession,
    state: &mut SessionState,
    participant_id: Option<Uuid>,
    was_host: bool,
) -> Result<()> {
    tracing::warn!("⏰ Peer timed out after grace period");

    if was_host && !state.is_host() {
        tracing::warn!("⚠️  Host timed out! Initiating host delegation...");

        let lobby = state
            .lobby_mut()
            .ok_or_else(|| CliError::InvalidConfig("No lobby".to_string()))?;

        // First, remove the timed-out host from participants (if they have a participant ID)
        if let Some(pid) = participant_id {
            if lobby.participants_mut().remove(&pid).is_none() {
                tracing::debug!("Participant {} was already removed", pid);
            } else {
                tracing::debug!("Removed timed-out host participant {}", pid);
            }
        }

        // Now try to auto-delegate to remaining guests
        match lobby.auto_delegate_host() {
            Ok(new_host_id) => {
                if new_host_id == state.participant().id() {
                    // We became the host!
                    tracing::info!("👑 You are now the HOST!");
                    state.promote_to_host();

                    // Broadcast the delegation event to all peers
                    let delegation_msg = DomainEvent::HostDelegated {
                        from: participant_id.unwrap_or_else(Uuid::new_v4),
                        to: new_host_id,
                        reason: konnekt_session_p2p::DelegationReason::Timeout,
                    };

                    session
                        .create_event(delegation_msg)
                        .map_err(|e| CliError::MessageSend(e.to_string()))?;

                    tracing::info!("📤 Broadcast host delegation event");
                } else {
                    tracing::info!("ℹ️  Guest '{}' became the new host", new_host_id);
                }

                // Clear disconnect timer (no longer needed)
                state.clear_host_disconnect_timer();
            }
            Err(e) => {
                tracing::warn!("❌ Failed to delegate host: {:?}", e);
            }
        }
    } else if let Some(pid) = participant_id {
        // Regular guest timed out
        tracing::info!("Guest {} timed out, removing from lobby", pid);

        // If we're the host, broadcast GuestLeft event
        if state.is_host() {
            // Remove from our local lobby
            if let Some(lobby) = state.lobby_mut() {
                if lobby.participants_mut().remove(&pid).is_none() {
                    tracing::debug!("Participant {} was already removed", pid);
                } else {
                    tracing::info!("Removed timed-out guest {}", pid);

                    // Broadcast to other guests
                    let event = DomainEvent::GuestLeft {
                        participant_id: pid,
                    };

                    session
                        .create_event(event)
                        .map_err(|e| CliError::MessageSend(e.to_string()))?;

                    tracing::info!("📤 Broadcast GuestLeft event");
                }
            }
        } else {
            // We're a guest, just remove from our local state
            // (we should receive the event from the host)
            if let Some(lobby) = state.lobby_mut() {
                lobby.participants_mut().remove(&pid);
            }
        }
    }

    Ok(())
}
