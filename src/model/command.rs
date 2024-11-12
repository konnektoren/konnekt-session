use crate::model::{ActivityStatus, Role};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ActivityId, PlayerId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LobbyCommand {
    Join {
        player_id: Uuid,
        lobby_id: Uuid,
        role: Role,
        data: String,
        password: Option<String>,
    },
    ParticipantInfo {
        player_id: PlayerId,
        role: Role,
        data: String,
    },
    ActivityInfo {
        activity_id: ActivityId,
        status: ActivityStatus,
        data: String,
    },
    SelectActivity {
        activity_id: ActivityId,
    },
    AddParticipant {
        participant_id: PlayerId,
    },
    RemoveParticipant {
        participant_id: PlayerId,
    },
    StartActivity {
        activity_id: ActivityId,
    },
    CompleteActivity {
        activity_id: ActivityId,
    },
    AddActivityResult {
        activity_id: ActivityId,
        player_id: PlayerId,
        data: String,
    },
    UpdateActivityStatus {
        activity_id: ActivityId,
        status: ActivityStatus,
    },
    UpdatePlayerId {
        player_id: PlayerId,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LobbyCommandWrapper {
    pub lobby_id: Uuid,
    pub password: Option<String>,
    pub command: LobbyCommand,
}

#[derive(Debug)]
pub enum CommandError {
    ActivityNotFound(String),
    ParticipantNotFound(Uuid),
    NotAuthorized,
    InvalidOperation(String),
}
