use crate::domain::{ActivityConfig, ActivityResult, ActivityRunId, ParticipationMode, RunStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DomainEvent {
    // ── Lobby events ─────────────────────────────────────────────────────────
    ParticipantJoined {
        lobby_id: Uuid,
        participant: crate::domain::Participant,
    },

    ParticipantLeft {
        lobby_id: Uuid,
        participant_id: Uuid,
    },

    ParticipantKicked {
        lobby_id: Uuid,
        participant_id: Uuid,
    },

    HostDelegated {
        lobby_id: Uuid,
        old_host_id: Uuid,
        new_host_id: Uuid,
    },

    ParticipationModeChanged {
        lobby_id: Uuid,
        participant_id: Uuid,
        new_mode: ParticipationMode,
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
}
