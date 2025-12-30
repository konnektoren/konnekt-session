use konnekt_session_core::{Lobby, ParticipationMode};
use konnekt_session_p2p::{DomainEvent as P2PDomainEvent, LobbyEvent};

/// Applies P2P events directly to domain state (for non-command events)
pub struct StateSynchronizer;

impl StateSynchronizer {
    /// Apply event directly to lobby state
    ///
    /// Returns true if state was modified
    pub fn apply_to_lobby(lobby: &mut Lobby, event: &LobbyEvent) -> Result<bool, String> {
        match &event.event {
            // GuestKicked - remove from participants
            P2PDomainEvent::GuestKicked { participant_id, .. } => {
                if lobby.participants_mut().remove(participant_id).is_some() {
                    Ok(true)
                } else {
                    Ok(false) // Already removed
                }
            }

            // ParticipationModeChanged - update mode
            P2PDomainEvent::ParticipationModeChanged {
                participant_id,
                new_mode,
            } => {
                let mode = if new_mode == "Active" {
                    ParticipationMode::Active
                } else {
                    ParticipationMode::Spectating
                };

                if let Some(participant) = lobby.participants_mut().get_mut(participant_id) {
                    participant.force_participation_mode(mode);
                    Ok(true)
                } else {
                    Err(format!("Participant {} not found", participant_id))
                }
            }

            // Other events go through command translation
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_core::Participant;
    use uuid::Uuid;

    #[test]
    fn test_apply_guest_kicked() {
        let host = Participant::new_host("Host".to_string()).unwrap();
        let mut lobby = Lobby::new("Test".to_string(), host).unwrap();

        let guest = Participant::new_guest("Guest".to_string()).unwrap();
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap();

        assert_eq!(lobby.participants().len(), 2);

        // Apply GuestKicked event
        let event = LobbyEvent::new(
            1,
            lobby.id(),
            P2PDomainEvent::GuestKicked {
                participant_id: guest_id,
                kicked_by: lobby.host_id(),
            },
        );

        let modified = StateSynchronizer::apply_to_lobby(&mut lobby, &event).unwrap();
        assert!(modified);
        assert_eq!(lobby.participants().len(), 1);
    }

    #[test]
    fn test_apply_participation_mode_changed() {
        let host = Participant::new_host("Host".to_string()).unwrap();
        let mut lobby = Lobby::new("Test".to_string(), host).unwrap();

        let guest = Participant::new_guest("Guest".to_string()).unwrap();
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap();

        // Guest starts in Active mode
        assert_eq!(
            lobby
                .participants()
                .get(&guest_id)
                .unwrap()
                .participation_mode(),
            ParticipationMode::Active
        );

        // Apply mode change
        let event = LobbyEvent::new(
            1,
            lobby.id(),
            P2PDomainEvent::ParticipationModeChanged {
                participant_id: guest_id,
                new_mode: "Spectating".to_string(),
            },
        );

        let modified = StateSynchronizer::apply_to_lobby(&mut lobby, &event).unwrap();
        assert!(modified);

        assert_eq!(
            lobby
                .participants()
                .get(&guest_id)
                .unwrap()
                .participation_mode(),
            ParticipationMode::Spectating
        );
    }

    #[test]
    fn test_apply_to_nonexistent_participant() {
        let host = Participant::new_host("Host".to_string()).unwrap();
        let mut lobby = Lobby::new("Test".to_string(), host).unwrap();

        let fake_id = Uuid::new_v4();
        let event = LobbyEvent::new(
            1,
            lobby.id(),
            P2PDomainEvent::ParticipationModeChanged {
                participant_id: fake_id,
                new_mode: "Spectating".to_string(),
            },
        );

        let result = StateSynchronizer::apply_to_lobby(&mut lobby, &event);
        assert!(result.is_err());
    }
}
