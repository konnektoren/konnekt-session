use crate::model::{
    ActivityData, CommandError, Lobby, LobbyCommand, LobbyCommandHandler, Player, PlayerData,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct LocalLobbyCommandHandler<P: PlayerData> {
    player_data_deserializer: Arc<dyn Fn(&str) -> P + Send + Sync>,
}

impl<P> LocalLobbyCommandHandler<P>
where
    P: PlayerData,
{
    pub fn new(deserializer: impl Fn(&str) -> P + Send + Sync + 'static) -> Self {
        Self {
            player_data_deserializer: Arc::new(deserializer),
        }
    }
}

impl<P, A> LobbyCommandHandler<P, A> for LocalLobbyCommandHandler<P>
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
            LobbyCommand::Join {
                player_id,
                role,
                data,
                password,
            } => {
                let data = (self.player_data_deserializer)(&data);
                let player: Player<P> = Player::new(role, data);

                lobby.join(player, password).unwrap();
                Ok(())
            }
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

    fn send_command(&self, _command: LobbyCommand) -> Result<(), CommandError> {
        // Implementation needed when you have to send the command to a remote server
        Ok(())
    }
}
