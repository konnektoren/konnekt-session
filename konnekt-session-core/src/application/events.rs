use crate::domain::{ActivityConfig, ActivityResult, ActivityRunId, Lobby, Participant, RunStatus};
use uuid::Uuid;

/// Events emitted by the domain after successful command execution
#[derive(Debug, Clone, PartialEq)]
pub enum DomainEvent {
    // ── Lobby events ─────────────────────────────────────────────────────────

    LobbyCreated { lobby: Lobby },

    GuestJoined {
        lobby_id: Uuid,
        participant: Participant,
    },

    GuestLeft {
        lobby_id: Uuid,
        participant_id: Uuid,
    },

    GuestKicked {
        lobby_id: Uuid,
        participant_id: Uuid,
        kicked_by: Uuid,
    },

    ParticipationModeChanged {
        lobby_id: Uuid,
        participant_id: Uuid,
        new_mode: crate::domain::ParticipationMode,
    },

    HostDelegated {
        lobby_id: Uuid,
        from: Uuid,
        to: Uuid,
    },

    ActivityQueued {
        lobby_id: Uuid,
        config: ActivityConfig,
    },

    // ── Run events ────────────────────────────────────────────────────────────

    RunStarted {
        lobby_id: Uuid,
        run_id: ActivityRunId,
        config: ActivityConfig,
    },

    ResultSubmitted {
        lobby_id: Uuid,
        run_id: ActivityRunId,
        result: ActivityResult,
    },

    SubmitterRemoved {
        lobby_id: Uuid,
        run_id: ActivityRunId,
        participant_id: Uuid,
    },

    RunEnded {
        lobby_id: Uuid,
        run_id: ActivityRunId,
        status: RunStatus,
        results: Vec<ActivityResult>,
    },

    // ── Errors ────────────────────────────────────────────────────────────────

    CommandFailed { command: String, reason: String },
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
    }
}
