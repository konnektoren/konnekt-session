use konnekt_session_core::{DomainCommand, DomainEvent};
use konnekt_session_p2p::{DomainEvent as P2PDomainEvent, LobbyEvent};
use uuid::Uuid;

/// Anti-Corruption Layer: Translates between P2P and Domain contexts
///
/// This protects the domain from P2P infrastructure concerns and vice versa.
pub struct MessageTranslator;

impl MessageTranslator {
    /// Translate P2P LobbyEvent to Domain Command
    ///
    /// Returns None if the event doesn't map to a command (e.g., LobbyCreated)
    pub fn to_domain_command(event: &LobbyEvent) -> Option<DomainCommand> {
        match &event.event {
            // GuestJoined → JoinLobby command
            P2PDomainEvent::GuestJoined { participant } => Some(DomainCommand::JoinLobby {
                lobby_id: event.lobby_id,
                guest_name: participant.name().to_string(),
            }),

            // GuestLeft → LeaveLobby command
            P2PDomainEvent::GuestLeft { participant_id } => Some(DomainCommand::LeaveLobby {
                lobby_id: event.lobby_id,
                participant_id: *participant_id,
            }),

            // GuestKicked → KickGuest command (already executed by host)
            P2PDomainEvent::GuestKicked {
                participant_id,
                kicked_by,
            } => {
                // This is a notification of an already-executed command
                // Guests apply this directly to their state
                // We might skip command generation here
                None // Or return a special "ApplyEvent" command
            }

            // HostDelegated → DelegateHost command
            P2PDomainEvent::HostDelegated { from, to, .. } => Some(DomainCommand::DelegateHost {
                lobby_id: event.lobby_id,
                current_host_id: *from,
                new_host_id: *to,
            }),

            // ParticipationModeChanged → ToggleParticipationMode command
            P2PDomainEvent::ParticipationModeChanged { participant_id, .. } => {
                // This is tricky - we don't know who requested it or if activity is in progress
                // We'll handle this differently - direct state application
                None
            }

            // LobbyCreated - this doesn't become a command (it's a creation event)
            P2PDomainEvent::LobbyCreated { .. } => None,
        }
    }

    /// Translate Domain Event to P2P LobbyEvent
    ///
    /// Adds sequence number and wraps in LobbyEvent envelope
    pub fn to_lobby_event(sequence: u64, lobby_id: Uuid, event: DomainEvent) -> LobbyEvent {
        let p2p_event = match event {
            DomainEvent::LobbyCreated { lobby } => P2PDomainEvent::LobbyCreated {
                lobby_id: lobby.id(),
                host_id: lobby.host_id(),
                name: lobby.name().to_string(),
            },

            DomainEvent::GuestJoined {
                lobby_id: _,
                participant,
            } => P2PDomainEvent::GuestJoined { participant },

            DomainEvent::GuestLeft {
                lobby_id: _,
                participant_id,
            } => P2PDomainEvent::GuestLeft { participant_id },

            DomainEvent::GuestKicked {
                lobby_id: _,
                participant_id,
                kicked_by,
            } => P2PDomainEvent::GuestKicked {
                participant_id,
                kicked_by,
            },

            DomainEvent::HostDelegated {
                lobby_id: _,
                from,
                to,
            } => P2PDomainEvent::HostDelegated {
                from,
                to,
                reason: konnekt_session_p2p::DelegationReason::Manual, // Default to manual
            },

            DomainEvent::ParticipationModeChanged {
                lobby_id: _,
                participant_id,
                new_mode,
            } => P2PDomainEvent::ParticipationModeChanged {
                participant_id,
                new_mode: format!("{}", new_mode),
            },

            DomainEvent::CommandFailed { .. } => {
                // Don't broadcast failures - they're local errors
                panic!("CommandFailed should not be broadcast");
            }
        };

        LobbyEvent::new(sequence, lobby_id, p2p_event)
    }

    /// Check if event should be broadcast (not all events are)
    pub fn should_broadcast(event: &DomainEvent) -> bool {
        !matches!(event, DomainEvent::CommandFailed { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_core::Participant;

    #[test]
    fn test_guest_joined_to_command() {
        let lobby_id = Uuid::new_v4();
        let participant = Participant::new_guest("Alice".to_string()).unwrap();

        let lobby_event = LobbyEvent::new(
            1,
            lobby_id,
            P2PDomainEvent::GuestJoined {
                participant: participant.clone(),
            },
        );

        let command = MessageTranslator::to_domain_command(&lobby_event);

        match command {
            Some(DomainCommand::JoinLobby {
                lobby_id: lid,
                guest_name,
            }) => {
                assert_eq!(lid, lobby_id);
                assert_eq!(guest_name, "Alice");
            }
            _ => panic!("Expected JoinLobby command"),
        }
    }

    #[test]
    fn test_guest_left_to_command() {
        let lobby_id = Uuid::new_v4();
        let participant_id = Uuid::new_v4();

        let lobby_event =
            LobbyEvent::new(1, lobby_id, P2PDomainEvent::GuestLeft { participant_id });

        let command = MessageTranslator::to_domain_command(&lobby_event);

        match command {
            Some(DomainCommand::LeaveLobby {
                lobby_id: lid,
                participant_id: pid,
            }) => {
                assert_eq!(lid, lobby_id);
                assert_eq!(pid, participant_id);
            }
            _ => panic!("Expected LeaveLobby command"),
        }
    }

    #[test]
    fn test_host_delegated_to_command() {
        let lobby_id = Uuid::new_v4();
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();

        let lobby_event = LobbyEvent::new(
            1,
            lobby_id,
            P2PDomainEvent::HostDelegated {
                from,
                to,
                reason: konnekt_session_p2p::DelegationReason::Manual,
            },
        );

        let command = MessageTranslator::to_domain_command(&lobby_event);

        match command {
            Some(DomainCommand::DelegateHost {
                lobby_id: lid,
                current_host_id,
                new_host_id,
            }) => {
                assert_eq!(lid, lobby_id);
                assert_eq!(current_host_id, from);
                assert_eq!(new_host_id, to);
            }
            _ => panic!("Expected DelegateHost command"),
        }
    }

    #[test]
    fn test_lobby_created_no_command() {
        let lobby_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();

        let lobby_event = LobbyEvent::new(
            1,
            lobby_id,
            P2PDomainEvent::LobbyCreated {
                lobby_id,
                host_id,
                name: "Test".to_string(),
            },
        );

        let command = MessageTranslator::to_domain_command(&lobby_event);
        assert!(command.is_none());
    }

    #[test]
    fn test_domain_event_to_lobby_event() {
        let lobby_id = Uuid::new_v4();
        let participant = Participant::new_guest("Bob".to_string()).unwrap();

        let domain_event = DomainEvent::GuestJoined {
            lobby_id,
            participant: participant.clone(),
        };

        let lobby_event = MessageTranslator::to_lobby_event(5, lobby_id, domain_event);

        assert_eq!(lobby_event.sequence, 5);
        assert_eq!(lobby_event.lobby_id, lobby_id);

        match lobby_event.event {
            P2PDomainEvent::GuestJoined { participant: p } => {
                assert_eq!(p.name(), "Bob");
            }
            _ => panic!("Expected GuestJoined"),
        }
    }

    #[test]
    fn test_should_broadcast() {
        let lobby_id = Uuid::new_v4();
        let participant = Participant::new_guest("Alice".to_string()).unwrap();

        let broadcastable = DomainEvent::GuestJoined {
            lobby_id,
            participant,
        };
        assert!(MessageTranslator::should_broadcast(&broadcastable));

        let not_broadcastable = DomainEvent::CommandFailed {
            command: "Test".to_string(),
            reason: "Error".to_string(),
        };
        assert!(!MessageTranslator::should_broadcast(&not_broadcastable));
    }

    #[test]
    #[should_panic(expected = "CommandFailed should not be broadcast")]
    fn test_command_failed_panics() {
        let lobby_id = Uuid::new_v4();
        let event = DomainEvent::CommandFailed {
            command: "Test".to_string(),
            reason: "Error".to_string(),
        };

        MessageTranslator::to_lobby_event(1, lobby_id, event);
    }
}
