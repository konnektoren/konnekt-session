use konnekt_session_core::{
    Participant, Timestamp,
    domain::{ActivityConfig, ActivityResult, ActivityRunId, RunStatus},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DomainEvent {
    // ── Lobby events ─────────────────────────────────────────────────────────
    LobbyCreated {
        lobby_id: Uuid,
        host_id: Uuid,
        name: String,
    },

    GuestJoined {
        participant: Participant,
    },

    GuestLeft {
        participant_id: Uuid,
    },

    GuestKicked {
        participant_id: Uuid,
        kicked_by: Uuid,
    },

    HostDelegated {
        from: Uuid,
        to: Uuid,
        reason: DelegationReason,
    },

    ParticipationModeChanged {
        participant_id: Uuid,
        new_mode: String,
    },

    ActivityQueued {
        config: ActivityConfig,
    },

    // ── Run events ────────────────────────────────────────────────────────────
    /// Host broadcasts when a run starts. Includes required_submitters so
    /// peers can independently track completion.
    RunStarted {
        run_id: ActivityRunId,
        config: ActivityConfig,
        required_submitters: Vec<Uuid>,
    },

    ResultSubmitted {
        run_id: ActivityRunId,
        result: ActivityResult,
    },

    RunEnded {
        run_id: ActivityRunId,
        status: RunStatus,
        results: Vec<ActivityResult>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DelegationReason {
    Manual,
    Timeout,
    Disconnect,
}

/// An event with metadata for ordering and synchronization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub struct LobbyEvent {
    pub sequence: u64,
    pub lobby_id: Uuid,
    pub timestamp: Timestamp,
    pub event: DomainEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<u8>>,
}

impl LobbyEvent {
    pub fn new(sequence: u64, lobby_id: Uuid, event: DomainEvent) -> Self {
        Self {
            sequence,
            lobby_id,
            timestamp: Timestamp::now(),
            event,
            signature: None,
        }
    }

    pub fn without_sequence(lobby_id: Uuid, event: DomainEvent) -> Self {
        Self {
            sequence: 0,
            lobby_id,
            timestamp: Timestamp::now(),
            event,
            signature: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = LobbyEvent::new(
            1,
            Uuid::new_v4(),
            DomainEvent::LobbyCreated {
                lobby_id: Uuid::new_v4(),
                host_id: Uuid::new_v4(),
                name: "Test Lobby".to_string(),
            },
        );

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: LobbyEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.sequence, deserialized.sequence);
        assert_eq!(event.lobby_id, deserialized.lobby_id);
    }

    #[test]
    fn test_domain_event_variants() {
        let guest_joined = DomainEvent::GuestJoined {
            participant: Participant::new_guest("Alice".to_string()).unwrap(),
        };

        let json = serde_json::to_string(&guest_joined).unwrap();
        assert!(json.contains("guest_joined"));

        let deserialized: DomainEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, DomainEvent::GuestJoined { .. }));
    }

    #[test]
    fn test_delegation_reason_serialization() {
        let manual = DelegationReason::Manual;
        let json = serde_json::to_string(&manual).unwrap();
        assert_eq!(json, "\"manual\"");
    }

    #[test]
    fn test_run_started_includes_submitters() {
        let run_id = Uuid::new_v4();
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let config =
            ActivityConfig::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));

        let event = DomainEvent::RunStarted {
            run_id,
            config,
            required_submitters: vec![p1, p2],
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: DomainEvent = serde_json::from_str(&json).unwrap();

        match deserialized {
            DomainEvent::RunStarted {
                required_submitters,
                ..
            } => {
                assert_eq!(required_submitters.len(), 2);
            }
            _ => panic!("Expected RunStarted"),
        }
    }
}
