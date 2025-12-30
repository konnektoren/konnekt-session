use konnekt_session_core::{DomainCommand, DomainEvent as CoreDomainEvent, ParticipationMode};
use uuid::Uuid;

use crate::domain::{DelegationReason, DomainEvent as P2PDomainEvent};

/// Translates between P2P domain events and Core domain commands/events
///
/// This is the Anti-Corruption Layer (ACL) that keeps the two bounded contexts separate.
#[derive(Debug, Clone)]
pub struct EventTranslator {
    /// Session/Lobby ID (1:1 relationship enforced by design)
    lobby_id: Uuid,
}

impl EventTranslator {
    /// Create a new translator for a specific lobby
    pub fn new(lobby_id: Uuid) -> Self {
        Self { lobby_id }
    }

    /// Translate a P2P domain event to a Core domain command
    ///
    /// Returns `None` if the event doesn't map to a command (e.g., LobbyCreated is a state snapshot)
    pub fn to_domain_command(&self, event: &P2PDomainEvent) -> Option<DomainCommand> {
        match event {
            P2PDomainEvent::GuestJoined { participant } => Some(DomainCommand::JoinLobby {
                lobby_id: self.lobby_id,
                guest_name: participant.name().to_string(),
            }),

            P2PDomainEvent::GuestLeft { participant_id } => Some(DomainCommand::LeaveLobby {
                lobby_id: self.lobby_id,
                participant_id: *participant_id,
            }),

            P2PDomainEvent::GuestKicked {
                participant_id,
                kicked_by,
            } => Some(DomainCommand::KickGuest {
                lobby_id: self.lobby_id,
                host_id: *kicked_by,
                guest_id: *participant_id,
            }),

            P2PDomainEvent::HostDelegated { from, to, .. } => Some(DomainCommand::DelegateHost {
                lobby_id: self.lobby_id,
                current_host_id: *from,
                new_host_id: *to,
            }),

            P2PDomainEvent::ParticipationModeChanged { participant_id, .. } => {
                // Note: We can't determine if activity is in progress from just the event
                // This will be handled by the domain layer's validation
                Some(DomainCommand::ToggleParticipationMode {
                    lobby_id: self.lobby_id,
                    participant_id: *participant_id,
                    requester_id: *participant_id, // Self-toggle
                    activity_in_progress: false,   // Will be validated by domain
                })
            }

            // LobbyCreated doesn't become a command - it's a state initialization event
            P2PDomainEvent::LobbyCreated { .. } => None,
        }
    }

    /// Translate a Core domain event to a P2P domain event
    ///
    /// Returns `None` for events that shouldn't be broadcast (e.g., CommandFailed)
    pub fn to_p2p_event(&self, event: CoreDomainEvent) -> Option<P2PDomainEvent> {
        match event {
            CoreDomainEvent::LobbyCreated { lobby } => Some(P2PDomainEvent::LobbyCreated {
                lobby_id: lobby.id(),
                host_id: lobby.host_id(),
                name: lobby.name().to_string(),
            }),

            CoreDomainEvent::GuestJoined {
                participant,
                lobby_id: _,
            } => Some(P2PDomainEvent::GuestJoined { participant }),

            CoreDomainEvent::GuestLeft {
                participant_id,
                lobby_id: _,
            } => Some(P2PDomainEvent::GuestLeft { participant_id }),

            CoreDomainEvent::GuestKicked {
                participant_id,
                kicked_by,
                lobby_id: _,
            } => Some(P2PDomainEvent::GuestKicked {
                participant_id,
                kicked_by,
            }),

            CoreDomainEvent::HostDelegated {
                from,
                to,
                lobby_id: _,
            } => Some(P2PDomainEvent::HostDelegated {
                from,
                to,
                reason: DelegationReason::Manual, // Default to manual for now
            }),

            CoreDomainEvent::ParticipationModeChanged {
                participant_id,
                new_mode,
                lobby_id: _,
            } => Some(P2PDomainEvent::ParticipationModeChanged {
                participant_id,
                new_mode: format!("{}", new_mode),
            }),

            // Don't broadcast failures - they're local errors
            CoreDomainEvent::CommandFailed { .. } => None,
        }
    }

    /// Get the lobby ID this translator is bound to
    pub fn lobby_id(&self) -> Uuid {
        self.lobby_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_core::Participant;

    #[test]
    fn test_guest_joined_to_command() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let participant = Participant::new_guest("Alice".to_string()).unwrap();
        let p2p_event = P2PDomainEvent::GuestJoined {
            participant: participant.clone(),
        };

        let command = translator.to_domain_command(&p2p_event);

        match command {
            Some(DomainCommand::JoinLobby {
                lobby_id: lid,
                guest_name,
            }) => {
                assert_eq!(lid, lobby_id);
                assert_eq!(guest_name, "Alice");
            }
            _ => panic!("Expected JoinLobby command, got: {:?}", command),
        }
    }

    #[test]
    fn test_guest_left_to_command() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);
        let participant_id = Uuid::new_v4();

        let p2p_event = P2PDomainEvent::GuestLeft { participant_id };

        let command = translator.to_domain_command(&p2p_event);

        match command {
            Some(DomainCommand::LeaveLobby {
                lobby_id: lid,
                participant_id: pid,
            }) => {
                assert_eq!(lid, lobby_id);
                assert_eq!(pid, participant_id);
            }
            _ => panic!("Expected LeaveLobby command, got: {:?}", command),
        }
    }

    #[test]
    fn test_guest_kicked_to_command() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);
        let participant_id = Uuid::new_v4();
        let kicked_by = Uuid::new_v4();

        let p2p_event = P2PDomainEvent::GuestKicked {
            participant_id,
            kicked_by,
        };

        let command = translator.to_domain_command(&p2p_event);

        match command {
            Some(DomainCommand::KickGuest {
                lobby_id: lid,
                host_id,
                guest_id,
            }) => {
                assert_eq!(lid, lobby_id);
                assert_eq!(host_id, kicked_by);
                assert_eq!(guest_id, participant_id);
            }
            _ => panic!("Expected KickGuest command, got: {:?}", command),
        }
    }

    #[test]
    fn test_host_delegated_to_command() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();

        let p2p_event = P2PDomainEvent::HostDelegated {
            from,
            to,
            reason: DelegationReason::Manual,
        };

        let command = translator.to_domain_command(&p2p_event);

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
            _ => panic!("Expected DelegateHost command, got: {:?}", command),
        }
    }

    #[test]
    fn test_participation_mode_changed_to_command() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);
        let participant_id = Uuid::new_v4();

        let p2p_event = P2PDomainEvent::ParticipationModeChanged {
            participant_id,
            new_mode: "Spectating".to_string(),
        };

        let command = translator.to_domain_command(&p2p_event);

        match command {
            Some(DomainCommand::ToggleParticipationMode {
                lobby_id: lid,
                participant_id: pid,
                requester_id: rid,
                activity_in_progress,
            }) => {
                assert_eq!(lid, lobby_id);
                assert_eq!(pid, participant_id);
                assert_eq!(rid, participant_id); // Self-toggle
                assert!(!activity_in_progress); // Default
            }
            _ => panic!(
                "Expected ToggleParticipationMode command, got: {:?}",
                command
            ),
        }
    }

    #[test]
    fn test_lobby_created_no_command() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let p2p_event = P2PDomainEvent::LobbyCreated {
            lobby_id,
            host_id: Uuid::new_v4(),
            name: "Test Lobby".to_string(),
        };

        let command = translator.to_domain_command(&p2p_event);
        assert!(command.is_none());
    }

    #[test]
    fn test_core_lobby_created_to_p2p() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let host = Participant::new_host("Host".to_string()).unwrap();
        let lobby =
            konnekt_session_core::Lobby::with_id(lobby_id, "Test".to_string(), host).unwrap();

        let core_event = CoreDomainEvent::LobbyCreated { lobby };

        let p2p_event = translator.to_p2p_event(core_event);

        match p2p_event {
            Some(P2PDomainEvent::LobbyCreated {
                lobby_id: lid,
                host_id,
                name,
            }) => {
                assert_eq!(lid, lobby_id);
                assert_eq!(name, "Test");
                assert_ne!(host_id, Uuid::nil());
            }
            _ => panic!("Expected LobbyCreated event, got: {:?}", p2p_event),
        }
    }

    #[test]
    fn test_core_guest_joined_to_p2p() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let participant = Participant::new_guest("Bob".to_string()).unwrap();
        let core_event = CoreDomainEvent::GuestJoined {
            lobby_id,
            participant: participant.clone(),
        };

        let p2p_event = translator.to_p2p_event(core_event);

        match p2p_event {
            Some(P2PDomainEvent::GuestJoined { participant: p }) => {
                assert_eq!(p.name(), "Bob");
            }
            _ => panic!("Expected GuestJoined event, got: {:?}", p2p_event),
        }
    }

    #[test]
    fn test_core_command_failed_no_p2p_event() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let core_event = CoreDomainEvent::CommandFailed {
            command: "Test".to_string(),
            reason: "Error".to_string(),
        };

        let p2p_event = translator.to_p2p_event(core_event);
        assert!(p2p_event.is_none());
    }

    #[test]
    fn test_participation_mode_serialization() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);
        let participant_id = Uuid::new_v4();

        let core_event = CoreDomainEvent::ParticipationModeChanged {
            lobby_id,
            participant_id,
            new_mode: ParticipationMode::Spectating,
        };

        let p2p_event = translator.to_p2p_event(core_event);

        match p2p_event {
            Some(P2PDomainEvent::ParticipationModeChanged {
                participant_id: pid,
                new_mode,
            }) => {
                assert_eq!(pid, participant_id);
                assert_eq!(new_mode, "Spectating");
            }
            _ => panic!(
                "Expected ParticipationModeChanged event, got: {:?}",
                p2p_event
            ),
        }
    }

    #[test]
    fn test_roundtrip_guest_operations() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        // Test: GuestJoined -> Command -> (would produce) -> GuestJoined
        let participant = Participant::new_guest("Charlie".to_string()).unwrap();
        let original_p2p = P2PDomainEvent::GuestJoined {
            participant: participant.clone(),
        };

        let command = translator.to_domain_command(&original_p2p).unwrap();

        // Simulate domain processing
        let core_event = CoreDomainEvent::GuestJoined {
            lobby_id,
            participant: participant.clone(),
        };

        let final_p2p = translator.to_p2p_event(core_event).unwrap();

        match final_p2p {
            P2PDomainEvent::GuestJoined { participant: p } => {
                assert_eq!(p.name(), "Charlie");
            }
            _ => panic!("Roundtrip failed"),
        }
    }
}
