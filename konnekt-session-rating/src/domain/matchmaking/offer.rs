use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identifies both searches and the unique offer, including its game room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub proposer: Uuid,
    pub responder: Uuid,
    pub id: Uuid,
    pub room: Uuid,
}
