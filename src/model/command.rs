use crate::model::{ActivityStatus, Role};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        player_id: Uuid,
        role: Role,
        data: String,
    },
    SelectActivity {
        activity_id: String,
    },
    AddParticipant {
        participant_id: Uuid,
    },
    RemoveParticipant {
        participant_id: Uuid,
    },
    StartActivity {
        activity_id: String,
    },
    CompleteActivity {
        activity_id: String,
    },
    UpdateActivityStatus {
        activity_id: String,
        status: ActivityStatus,
    },
    UpdatePlayerId {
        player_id: Uuid,
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
