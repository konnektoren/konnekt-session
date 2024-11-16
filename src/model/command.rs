use super::{ActivityId, PlayerId};
use crate::model::{ActivityStatus, LobbyId, Role};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LobbyCommand {
    Join {
        player_id: PlayerId,
        lobby_id: LobbyId,
        role: Role,
        data: String,
        password: Option<String>,
    },
    PlayerInfo {
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
    RemovePlayer {
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
    pub lobby_id: LobbyId,
    pub password: Option<String>,
    pub command: LobbyCommand,
}

#[derive(Debug)]
pub enum CommandError {
    ActivityNotFound(String),
    PlayerNotFound(PlayerId),
    NotAuthorized,
    InvalidOperation(String),
}
