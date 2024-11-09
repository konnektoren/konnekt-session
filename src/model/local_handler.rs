use crate::model::{
    ActivityData, CommandError, Lobby, LobbyCommand, LobbyCommandHandler, PlayerData,
};

pub struct LocalLobbyCommandHandler;

impl<P, A> LobbyCommandHandler<P, A> for LocalLobbyCommandHandler
where
    P: PlayerData,
    A: ActivityData,
{
    fn handle_command(
        &self,
        lobby: &mut Lobby<P, A>,
        command: LobbyCommand,
    ) -> Result<(), CommandError> {
        match command {
            LobbyCommand::SelectActivity { activity_id } => {
                lobby
                    .select_activity(&activity_id)
                    .ok_or(CommandError::ActivityNotFound(activity_id))?;
                Ok(())
            }
            LobbyCommand::AddParticipant { participant_id } => {
                // Implementation needed when you have the participant data
                Ok(())
            }
            LobbyCommand::RemoveParticipant { participant_id } => {
                lobby
                    .remove_participant(participant_id)
                    .ok_or(CommandError::ParticipantNotFound(participant_id))?;
                Ok(())
            }
            LobbyCommand::StartActivity { activity_id } => {
                lobby
                    .start_activity(&activity_id)
                    .ok_or(CommandError::ActivityNotFound(activity_id))?;
                Ok(())
            }
            LobbyCommand::CompleteActivity { activity_id } => {
                lobby
                    .complete_activity(&activity_id)
                    .ok_or(CommandError::ActivityNotFound(activity_id))?;
                Ok(())
            }
            LobbyCommand::UpdateActivityStatus {
                activity_id,
                status,
            } => {
                lobby
                    .update_activity_status(&activity_id, status)
                    .ok_or(CommandError::ActivityNotFound(activity_id))?;
                Ok(())
            }
        }
    }
}
