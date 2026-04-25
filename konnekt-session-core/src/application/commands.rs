use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DomainCommand {
    // ── Lobby commands ────────────────────────────────────────────────────────

    CreateLobby {
        lobby_id: Option<Uuid>,
        lobby_name: String,
        host_name: String,
    },

    CreateLobbyWithHost {
        lobby_id: Uuid,
        lobby_name: String,
        host: crate::domain::Participant,
    },

    JoinLobby {
        lobby_id: Uuid,
        guest_name: String,
    },

    LeaveLobby {
        lobby_id: Uuid,
        participant_id: Uuid,
    },

    KickGuest {
        lobby_id: Uuid,
        host_id: Uuid,
        guest_id: Uuid,
    },

    /// `activity_in_progress` no longer needed — Lobby tracks this via `active_run_id`.
    ToggleParticipationMode {
        lobby_id: Uuid,
        participant_id: Uuid,
        requester_id: Uuid,
    },

    DelegateHost {
        lobby_id: Uuid,
        current_host_id: Uuid,
        new_host_id: Uuid,
    },

    /// Add a participant directly (P2P sync).
    AddParticipant {
        lobby_id: Uuid,
        participant: crate::domain::Participant,
    },

    /// Force-set a participant's mode (P2P sync).
    UpdateParticipantMode {
        lobby_id: Uuid,
        participant_id: Uuid,
        new_mode: crate::domain::ParticipationMode,
    },

    QueueActivity {
        lobby_id: Uuid,
        config: crate::domain::ActivityConfig,
    },

    // ── Run commands ──────────────────────────────────────────────────────────

    /// Dequeue the next activity and start a run.
    StartNextRun {
        lobby_id: Uuid,
    },

    SubmitResult {
        lobby_id: Uuid,
        run_id: crate::domain::ActivityRunId,
        result: crate::domain::ActivityResult,
    },

    CancelRun {
        lobby_id: Uuid,
        run_id: crate::domain::ActivityRunId,
    },

    /// Remove a participant from a run's required submitters (on disconnect).
    RemoveSubmitter {
        lobby_id: Uuid,
        run_id: crate::domain::ActivityRunId,
        participant_id: Uuid,
    },

    /// P2P sync: guest applies a run that the host already started.
    SyncRunStarted {
        lobby_id: Uuid,
        run_id: crate::domain::ActivityRunId,
        config: crate::domain::ActivityConfig,
        required_submitters: Vec<Uuid>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_clone() {
        let cmd = DomainCommand::CreateLobby {
            lobby_name: "Test".to_string(),
            host_name: "Alice".to_string(),
            lobby_id: None,
        };
        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }
}
