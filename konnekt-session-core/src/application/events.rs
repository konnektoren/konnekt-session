use crate::domain::{Lobby, Participant};
use uuid::Uuid;

/// Events emitted by the domain after successful command execution
#[derive(Debug, Clone, PartialEq)]
pub enum DomainEvent {
    /// Lobby was created
    LobbyCreated { lobby: Lobby },

    /// Guest joined the lobby
    GuestJoined {
        lobby_id: Uuid,
        participant: Participant,
    },

    /// Guest left the lobby
    GuestLeft {
        lobby_id: Uuid,
        participant_id: Uuid,
    },

    /// Guest was kicked by host
    GuestKicked {
        lobby_id: Uuid,
        participant_id: Uuid,
        kicked_by: Uuid,
    },

    /// Participation mode changed
    ParticipationModeChanged {
        lobby_id: Uuid,
        participant_id: Uuid,
        new_mode: crate::domain::ParticipationMode,
    },

    /// Host role was delegated
    HostDelegated {
        lobby_id: Uuid,
        from: Uuid,
        to: Uuid,
    },

    /// Command failed
    CommandFailed { command: String, reason: String },

    /// Activity was planned
    ActivityPlanned {
        lobby_id: Uuid,
        metadata: crate::domain::ActivityMetadata,
    },

    /// Activity was started
    ActivityStarted {
        lobby_id: Uuid,
        activity_id: crate::domain::ActivityId,
    },

    /// Result was submitted
    ResultSubmitted {
        lobby_id: Uuid,
        result: crate::domain::ActivityResult,
    },

    /// Activity completed
    ActivityCompleted {
        lobby_id: Uuid,
        activity_id: crate::domain::ActivityId,
        results: Vec<crate::domain::ActivityResult>,
    },

    /// Activity cancelled
    ActivityCancelled {
        lobby_id: Uuid,
        activity_id: crate::domain::ActivityId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ParticipationMode;

    #[test]
    fn test_event_clone() {
        let event = DomainEvent::GuestLeft {
            lobby_id: Uuid::new_v4(),
            participant_id: Uuid::new_v4(),
        };

        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn test_event_debug() {
        let event = DomainEvent::ParticipationModeChanged {
            lobby_id: Uuid::new_v4(),
            participant_id: Uuid::new_v4(),
            new_mode: ParticipationMode::Spectating,
        };

        let debug = format!("{:?}", event);
        assert!(debug.contains("ParticipationModeChanged"));
        assert!(debug.contains("Spectating"));
    }

    #[test]
    fn test_command_failed_event() {
        let event = DomainEvent::CommandFailed {
            command: "JoinLobby".to_string(),
            reason: "Lobby full".to_string(),
        };

        if let DomainEvent::CommandFailed { command, reason } = event {
            assert_eq!(command, "JoinLobby");
            assert_eq!(reason, "Lobby full");
        } else {
            panic!("Expected CommandFailed");
        }
    }
}
