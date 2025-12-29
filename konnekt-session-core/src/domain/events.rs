use crate::domain::ParticipationMode;
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
}
