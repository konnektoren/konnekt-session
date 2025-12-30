use konnekt_session_core::{Participant, Timestamp};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Domain event in the lobby
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DomainEvent {
    /// Lobby was created
    LobbyCreated {
        lobby_id: Uuid,
        host_id: Uuid,
        name: String,
    },

    /// Guest joined the lobby
    GuestJoined { participant: Participant },

    /// Guest left the lobby
    GuestLeft { participant_id: Uuid },

    /// Guest was kicked by host
    GuestKicked {
        participant_id: Uuid,
        kicked_by: Uuid,
    },

    /// Host role was delegated
    HostDelegated {
        from: Uuid,
        to: Uuid,
        reason: DelegationReason,
    },

    /// Participant changed participation mode
    ParticipationModeChanged {
        participant_id: Uuid,
        new_mode: String, // "Active" | "Spectating"
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationReason {
    Manual,
    Timeout,
}

/// An event with metadata for ordering and synchronization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LobbyEvent {
    /// Monotonically increasing sequence number (assigned by host)
    pub sequence: u64,

    /// Lobby this event belongs to
    pub lobby_id: Uuid,

    /// When this event was created
    pub timestamp: Timestamp,

    /// The actual domain event
    pub event: DomainEvent,

    /// Host's signature (TODO: in future commit)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Vec<u8>>,
}

impl LobbyEvent {
    /// Create a new event with sequence and timestamp
    pub fn new(sequence: u64, lobby_id: Uuid, event: DomainEvent) -> Self {
        Self {
            sequence,
            lobby_id,
            timestamp: Timestamp::now(),
            event,
            signature: None,
        }
    }

    /// Create an event without sequence (for guests creating requests)
    pub fn without_sequence(lobby_id: Uuid, event: DomainEvent) -> Self {
        Self {
            sequence: 0, // Will be assigned by host
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

        let timeout = DelegationReason::Timeout;
        let json = serde_json::to_string(&timeout).unwrap();
        assert_eq!(json, "\"timeout\"");
    }

    #[test]
    fn test_event_without_sequence() {
        let lobby_id = Uuid::new_v4();
        let event = LobbyEvent::without_sequence(
            lobby_id,
            DomainEvent::GuestLeft {
                participant_id: Uuid::new_v4(),
            },
        );

        assert_eq!(event.sequence, 0);
        assert_eq!(event.lobby_id, lobby_id);
    }
}
