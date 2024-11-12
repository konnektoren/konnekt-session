use crate::model::{
    Activity, ActivityData, ActivityResultData, CommandError, Lobby, LobbyCommand,
    LobbyCommandHandler, Player, PlayerData,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct LocalLobbyCommandHandler<P: PlayerData, A: ActivityData, AR: ActivityResultData> {
    player_data_deserializer: Arc<dyn Fn(&str) -> P + Send + Sync>,
    activity_data_deserializer: Arc<dyn Fn(&str) -> A + Send + Sync>,
    phantom: std::marker::PhantomData<AR>,
}

impl<P, A, AR> LocalLobbyCommandHandler<P, A, AR>
where
    P: PlayerData,
    A: ActivityData,
    AR: ActivityResultData,
{
    pub fn new(
        player_data_deserializer: impl Fn(&str) -> P + Send + Sync + 'static,
        activity_data_deserializer: impl Fn(&str) -> A + Send + Sync + 'static,
    ) -> Self {
        Self {
            player_data_deserializer: Arc::new(player_data_deserializer),
            activity_data_deserializer: Arc::new(activity_data_deserializer),
            phantom: std::marker::PhantomData,
        }
    }
}

impl<P, A, AR> LobbyCommandHandler<P, A, AR> for LocalLobbyCommandHandler<P, A, AR>
where
    P: PlayerData,
    A: ActivityData,
    AR: ActivityResultData,
{
    fn handle_command(
        &self,
        lobby: &mut Lobby<P, A, AR>,
        command: LobbyCommand,
    ) -> Result<(), CommandError> {
        match command {
            LobbyCommand::Join {
                player_id,
                role,
                data,
                password,
                ..
            } => {
                let data = (self.player_data_deserializer)(&data);
                let mut player: Player<P> = Player::new(role, data);
                player.id = player_id;
                lobby.join(player, password).unwrap();

                Ok(())
            }
            LobbyCommand::ParticipantInfo {
                player_id,
                role,
                data,
            } => {
                let data = (self.player_data_deserializer)(&data);
                let mut player: Player<P> = Player::new(role, data);
                player.id = player_id;
                lobby.add_participant(player);
                Ok(())
            }
            LobbyCommand::ActivityInfo {
                activity_id,
                status,
                data,
            } => {
                let data = (self.activity_data_deserializer)(&data);
                lobby.add_activity(Activity {
                    id: activity_id,
                    status,
                    data,
                });
                Ok(())
            }
            LobbyCommand::SelectActivity { activity_id } => {
                lobby
                    .select_activity(&activity_id)
                    .ok_or(CommandError::ActivityNotFound(activity_id))?;
                Ok(())
            }
            LobbyCommand::AddParticipant { .. } => {
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
            LobbyCommand::UpdatePlayerId { player_id } => {
                lobby.update_player_id(&player_id);
                Ok(())
            }
        }
    }

    fn send_command(&self, _command: LobbyCommand) -> Result<(), CommandError> {
        // Implementation needed when you have to send the command to a remote server
        Ok(())
    }
}
