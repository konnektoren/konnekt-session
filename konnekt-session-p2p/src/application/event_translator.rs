use konnekt_session_core::{DomainCommand, DomainEvent as CoreDomainEvent, ParticipationMode};
use uuid::Uuid;

use crate::domain::{DelegationReason, DomainEvent as P2PDomainEvent};

#[derive(Debug, Clone)]
pub struct EventTranslator {
    lobby_id: Uuid,
}

impl EventTranslator {
    pub fn new(lobby_id: Uuid) -> Self {
        Self { lobby_id }
    }

    /// Translate a P2P event received from a peer into a core domain command.
    /// Returns `None` for events that are state snapshots (not commands).
    pub fn to_domain_command(&self, event: &P2PDomainEvent) -> Option<DomainCommand> {
        match event {
            P2PDomainEvent::GuestLeft { participant_id } => Some(DomainCommand::LeaveLobby {
                lobby_id: self.lobby_id,
                participant_id: *participant_id,
            }),

            P2PDomainEvent::GuestKicked { participant_id, kicked_by } => {
                Some(DomainCommand::KickGuest {
                    lobby_id: self.lobby_id,
                    host_id: *kicked_by,
                    guest_id: *participant_id,
                })
            }

            P2PDomainEvent::HostDelegated { from, to, .. } => Some(DomainCommand::DelegateHost {
                lobby_id: self.lobby_id,
                current_host_id: *from,
                new_host_id: *to,
            }),

            P2PDomainEvent::ParticipationModeChanged { participant_id, new_mode } => {
                let mode = match new_mode.as_str() {
                    "Active" => ParticipationMode::Active,
                    "Spectating" => ParticipationMode::Spectating,
                    _ => {
                        tracing::warn!("Unknown participation mode: {}", new_mode);
                        return None;
                    }
                };
                Some(DomainCommand::UpdateParticipantMode {
                    lobby_id: self.lobby_id,
                    participant_id: *participant_id,
                    new_mode: mode,
                })
            }

            P2PDomainEvent::GuestJoined { participant } => Some(DomainCommand::AddParticipant {
                lobby_id: self.lobby_id,
                participant: participant.clone(),
            }),

            P2PDomainEvent::ActivityQueued { config } => Some(DomainCommand::QueueActivity {
                lobby_id: self.lobby_id,
                config: config.clone(),
            }),

            P2PDomainEvent::ResultSubmitted { run_id, result } => Some(DomainCommand::SubmitResult {
                lobby_id: self.lobby_id,
                run_id: *run_id,
                result: result.clone(),
            }),

            // State snapshots — applied via snapshot sync, not commands
            P2PDomainEvent::LobbyCreated { .. } => None,
            P2PDomainEvent::RunStarted { .. } => None,
            P2PDomainEvent::RunEnded { .. } => None,
        }
    }

    /// Translate a core domain event into a P2P event for broadcasting.
    /// Returns `None` for events that should not be broadcast.
    pub fn to_p2p_event(&self, event: CoreDomainEvent) -> Option<P2PDomainEvent> {
        match event {
            CoreDomainEvent::LobbyCreated { lobby } => Some(P2PDomainEvent::LobbyCreated {
                lobby_id: lobby.id(),
                host_id: lobby.host_id(),
                name: lobby.name().to_string(),
            }),

            CoreDomainEvent::GuestJoined { participant, .. } => {
                Some(P2PDomainEvent::GuestJoined { participant })
            }

            CoreDomainEvent::GuestLeft { participant_id, .. } => {
                Some(P2PDomainEvent::GuestLeft { participant_id })
            }

            CoreDomainEvent::GuestKicked { participant_id, kicked_by, .. } => {
                Some(P2PDomainEvent::GuestKicked { participant_id, kicked_by })
            }

            CoreDomainEvent::HostDelegated { from, to, .. } => {
                Some(P2PDomainEvent::HostDelegated {
                    from,
                    to,
                    reason: DelegationReason::Manual,
                })
            }

            CoreDomainEvent::ParticipationModeChanged { participant_id, new_mode, .. } => {
                Some(P2PDomainEvent::ParticipationModeChanged {
                    participant_id,
                    new_mode: format!("{}", new_mode),
                })
            }

            CoreDomainEvent::ActivityQueued { config, .. } => {
                Some(P2PDomainEvent::ActivityQueued { config })
            }

            CoreDomainEvent::RunStarted { run_id, config, .. } => {
                // required_submitters comes from the ActivityRun — caller must enrich this.
                // For now we broadcast without submitters; snapshot sync covers guests.
                Some(P2PDomainEvent::RunStarted {
                    run_id,
                    config,
                    required_submitters: vec![],
                })
            }

            CoreDomainEvent::ResultSubmitted { run_id, result, .. } => {
                Some(P2PDomainEvent::ResultSubmitted { run_id, result })
            }

            CoreDomainEvent::SubmitterRemoved { .. } => None,

            CoreDomainEvent::RunEnded { run_id, status, results, .. } => {
                Some(P2PDomainEvent::RunEnded { run_id, status, results })
            }

            CoreDomainEvent::CommandFailed { .. } => None,
        }
    }

    pub fn lobby_id(&self) -> Uuid {
        self.lobby_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_core::{
        Participant,
        domain::{ActivityConfig, ActivityResult},
    };

    #[test]
    fn test_core_lobby_created_to_p2p() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let host = Participant::new_host("Host".to_string()).unwrap();
        let lobby =
            konnekt_session_core::Lobby::with_id(lobby_id, "Test".to_string(), host).unwrap();

        let p2p_event = translator.to_p2p_event(CoreDomainEvent::LobbyCreated { lobby });

        match p2p_event {
            Some(P2PDomainEvent::LobbyCreated { lobby_id: lid, name, .. }) => {
                assert_eq!(lid, lobby_id);
                assert_eq!(name, "Test");
            }
            _ => panic!("Expected LobbyCreated, got: {:?}", p2p_event),
        }
    }

    #[test]
    fn test_command_failed_not_translated() {
        let translator = EventTranslator::new(Uuid::new_v4());
        let p2p_event = translator.to_p2p_event(CoreDomainEvent::CommandFailed {
            command: "Test".to_string(),
            reason: "Error".to_string(),
        });
        assert!(p2p_event.is_none());
    }

    #[test]
    fn test_activity_queued_roundtrip() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let config = ActivityConfig::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));

        let core_event = CoreDomainEvent::ActivityQueued { lobby_id, config: config.clone() };
        let p2p_event = translator.to_p2p_event(core_event).expect("Should translate");

        let command = translator.to_domain_command(&p2p_event).expect("Should map to command");

        match command {
            DomainCommand::QueueActivity { lobby_id: lid, config: c } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(c.activity_type, "quiz");
            }
            _ => panic!("Expected QueueActivity, got {:?}", command),
        }
    }

    #[test]
    fn test_result_submitted_translation() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let run_id = Uuid::new_v4();
        let participant_id = Uuid::new_v4();
        let result = ActivityResult::new(run_id, participant_id).with_score(100);

        let core_event = CoreDomainEvent::ResultSubmitted {
            lobby_id,
            run_id,
            result: result.clone(),
        };

        let p2p_event = translator.to_p2p_event(core_event).expect("Should translate");

        match &p2p_event {
            P2PDomainEvent::ResultSubmitted { run_id: rid, result: res } => {
                assert_eq!(*rid, run_id);
                assert_eq!(res.participant_id, participant_id);
                assert_eq!(res.score, Some(100));
            }
            _ => panic!("Expected ResultSubmitted"),
        }

        // P2P → Command
        let command = translator.to_domain_command(&p2p_event).expect("Should map");
        match command {
            DomainCommand::SubmitResult { run_id: rid, .. } => assert_eq!(rid, run_id),
            _ => panic!("Expected SubmitResult"),
        }
    }

    #[test]
    fn test_run_ended_not_a_command() {
        let translator = EventTranslator::new(Uuid::new_v4());
        let p2p_event = P2PDomainEvent::RunEnded {
            run_id: Uuid::new_v4(),
            status: konnekt_session_core::domain::RunStatus::Completed,
            results: vec![],
        };
        assert!(translator.to_domain_command(&p2p_event).is_none());
    }
}
