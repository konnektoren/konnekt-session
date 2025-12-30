use crate::domain::{ActivityId, ActivityMetadata, ActivityResult, ParticipationMode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Domain events that occur in the lobby
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DomainEvent {
    /// A participant's participation mode changed
    ParticipationModeChanged {
        participant_id: Uuid,
        new_mode: ParticipationMode,
        forced: bool, // true if changed by host, false if self-requested
    },

    /// Host planned a new activity
    ActivityPlanned {
        lobby_id: Uuid,
        metadata: ActivityMetadata, // Contains serialized config
    },

    /// Host started an activity
    ActivityStarted {
        lobby_id: Uuid,
        activity_id: ActivityId,
    },

    /// Participant submitted result
    ResultSubmitted {
        lobby_id: Uuid,
        result: ActivityResult, // Contains serialized result data
    },

    /// Activity completed (all active participants done)
    ActivityCompleted {
        lobby_id: Uuid,
        activity_id: ActivityId,
        results: Vec<ActivityResult>,
    },
}
