use konnekt_session_core::{DomainCommand, DomainEvent as CoreDomainEvent, ParticipationMode};
use uuid::Uuid;

use crate::domain::{DelegationReason, DomainEvent as P2PDomainEvent};

/// Translates between P2P domain events and Core domain commands/events
///
/// Enforces 1:1 mappings:
/// - Session ID ↔ Lobby ID (same UUID)
/// - Peer ID ↔ Participant ID (managed by PeerRegistry)
#[derive(Debug, Clone)]
pub struct EventTranslator {
    /// Lobby ID (same as Session ID - 1:1 relationship)
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
                Some(DomainCommand::ToggleParticipationMode {
                    lobby_id: self.lobby_id,
                    participant_id: *participant_id,
                    requester_id: *participant_id,
                    activity_in_progress: false,
                })
            }

            // ✅ ADD: Activity events → Commands
            P2PDomainEvent::ActivityPlanned { metadata } => Some(DomainCommand::PlanActivity {
                lobby_id: self.lobby_id,
                metadata: metadata.clone(),
            }),

            P2PDomainEvent::ActivityStarted { activity_id } => Some(DomainCommand::StartActivity {
                lobby_id: self.lobby_id,
                activity_id: *activity_id,
            }),

            P2PDomainEvent::ResultSubmitted { result } => Some(DomainCommand::SubmitResult {
                lobby_id: self.lobby_id,
                result: result.clone(),
            }),

            P2PDomainEvent::ActivityCancelled { activity_id } => {
                Some(DomainCommand::CancelActivity {
                    lobby_id: self.lobby_id,
                    activity_id: *activity_id,
                })
            }

            // State snapshots - don't map to commands
            P2PDomainEvent::LobbyCreated { .. } => None,
            P2PDomainEvent::ActivityCompleted { .. } => None, // Result of SubmitResult, not a command
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
                reason: DelegationReason::Manual,
            }),

            CoreDomainEvent::ParticipationModeChanged {
                participant_id,
                new_mode,
                lobby_id: _,
            } => Some(P2PDomainEvent::ParticipationModeChanged {
                participant_id,
                new_mode: format!("{}", new_mode),
            }),

            // ✅ ADD: Activity events
            CoreDomainEvent::ActivityPlanned {
                metadata,
                lobby_id: _,
            } => Some(P2PDomainEvent::ActivityPlanned { metadata }),

            CoreDomainEvent::ActivityStarted {
                activity_id,
                lobby_id: _,
            } => Some(P2PDomainEvent::ActivityStarted { activity_id }),

            CoreDomainEvent::ResultSubmitted {
                result,
                lobby_id: _,
            } => Some(P2PDomainEvent::ResultSubmitted { result }),

            CoreDomainEvent::ActivityCompleted {
                activity_id,
                results,
                lobby_id: _,
            } => Some(P2PDomainEvent::ActivityCompleted {
                activity_id,
                results,
            }),

            CoreDomainEvent::ActivityCancelled {
                activity_id,
                lobby_id: _,
            } => Some(P2PDomainEvent::ActivityCancelled { activity_id }),

            // Don't broadcast command failures
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
    use konnekt_session_core::{
        EchoChallenge, Participant,
        domain::{ActivityMetadata, ActivityResult},
    };

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
    fn test_command_failed_not_translated() {
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
    fn test_roundtrip_translation() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let participant = Participant::new_guest("Charlie".to_string()).unwrap();
        let original_core = CoreDomainEvent::GuestJoined {
            lobby_id,
            participant: participant.clone(),
        };

        // Core → P2P
        let p2p_event = translator
            .to_p2p_event(original_core.clone())
            .expect("Should translate to P2P");

        // P2P → Domain Command
        let domain_cmd = translator
            .to_domain_command(&p2p_event)
            .expect("Should translate to command");

        match domain_cmd {
            DomainCommand::JoinLobby {
                lobby_id: lid,
                guest_name,
            } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(guest_name, "Charlie");
            }
            _ => panic!("Expected JoinLobby command"),
        }
    }

    #[test]
    fn test_activity_planned_to_p2p() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let challenge = EchoChallenge::new("Test".to_string());
        let metadata = ActivityMetadata::new(
            "echo-challenge-v1".to_string(),
            "Echo Test".to_string(),
            challenge.to_config(),
        );

        let core_event = CoreDomainEvent::ActivityPlanned {
            lobby_id,
            metadata: metadata.clone(),
        };

        let p2p_event = translator
            .to_p2p_event(core_event)
            .expect("Should translate");

        match p2p_event {
            P2PDomainEvent::ActivityPlanned { metadata: meta } => {
                assert_eq!(meta.activity_type, "echo-challenge-v1");
                assert_eq!(meta.name, "Echo Test");
            }
            _ => panic!("Expected ActivityPlanned event"),
        }
    }

    #[test]
    fn test_activity_started_roundtrip() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);
        let activity_id = Uuid::new_v4();

        // Core → P2P
        let core_event = CoreDomainEvent::ActivityStarted {
            lobby_id,
            activity_id,
        };
        let p2p_event = translator
            .to_p2p_event(core_event)
            .expect("Should translate");

        // P2P → Command
        let command = translator
            .to_domain_command(&p2p_event)
            .expect("Should translate");

        match command {
            DomainCommand::StartActivity {
                lobby_id: lid,
                activity_id: aid,
            } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(aid, activity_id);
            }
            _ => panic!("Expected StartActivity command"),
        }
    }

    #[test]
    fn test_result_submitted_translation() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let activity_id = Uuid::new_v4();
        let participant_id = Uuid::new_v4();

        let result = ActivityResult::new(activity_id, participant_id).with_score(100);

        let core_event = CoreDomainEvent::ResultSubmitted {
            lobby_id,
            result: result.clone(),
        };

        let p2p_event = translator
            .to_p2p_event(core_event)
            .expect("Should translate");

        match p2p_event {
            P2PDomainEvent::ResultSubmitted { result: res } => {
                assert_eq!(res.activity_id, activity_id);
                assert_eq!(res.participant_id, participant_id);
                assert_eq!(res.score, Some(100));
            }
            _ => panic!("Expected ResultSubmitted event"),
        }
    }

    #[test]
    fn test_activity_completed_not_a_command() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let p2p_event = P2PDomainEvent::ActivityCompleted {
            activity_id: Uuid::new_v4(),
            results: vec![],
        };

        let command = translator.to_domain_command(&p2p_event);

        // ActivityCompleted is a state snapshot, not a command
        assert!(command.is_none());
    }

    #[test]
    fn test_activity_cancelled_roundtrip() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);
        let activity_id = Uuid::new_v4();

        // Core → P2P
        let core_event = CoreDomainEvent::ActivityCancelled {
            lobby_id,
            activity_id,
        };
        let p2p_event = translator
            .to_p2p_event(core_event)
            .expect("Should translate");

        // P2P → Command
        let command = translator
            .to_domain_command(&p2p_event)
            .expect("Should translate");

        match command {
            DomainCommand::CancelActivity {
                lobby_id: lid,
                activity_id: aid,
            } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(aid, activity_id);
            }
            _ => panic!("Expected CancelActivity command"),
        }
    }
}
