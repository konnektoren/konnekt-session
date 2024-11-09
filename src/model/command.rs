use crate::model::ActivityStatus;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum LobbyCommand {
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
}

#[derive(Debug)]
pub enum CommandError {
    ActivityNotFound(String),
    ParticipantNotFound(Uuid),
    NotAuthorized,
    InvalidOperation(String),
}
