use serde::{Deserialize, Serialize};
use std::rc::Rc;

use crate::model::{
    ActivityResult, ActivityResultTrait, ActivityTrait, CommandError, Lobby,
    LobbyCommand, LobbyCommandHandler, Player, PlayerTrait,
};

#[derive(Clone)]
pub struct LocalLobbyCommandHandler<P: PlayerTrait, A: ActivityTrait, AR: ActivityResultTrait> {
    player_data_deserializer: Rc<dyn Fn(&str) -> P>,
    activity_data_deserializer: Rc<dyn Fn(&str) -> A>,
    activity_result_data_deserializer: Rc<dyn Fn(&str) -> AR>,
}

impl<P: PlayerTrait, A: ActivityTrait, AR: ActivityResultTrait> PartialEq
    for LocalLobbyCommandHandler<P, A, AR>
{
    fn eq(&self, _other: &Self) -> bool {
        // Consider all handlers equal since we can't compare function pointers
        true
    }
}

impl<P, A, AR> LocalLobbyCommandHandler<P, A, AR>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static,
{
    pub fn new(
        player_data_deserializer: impl Fn(&str) -> P + 'static,
        activity_data_deserializer: impl Fn(&str) -> A + 'static,
        activity_result_data_deserializer: impl Fn(&str) -> AR + 'static,
    ) -> Self {
        Self {
            player_data_deserializer: Rc::new(player_data_deserializer),
            activity_data_deserializer: Rc::new(activity_data_deserializer),
            activity_result_data_deserializer: Rc::new(activity_result_data_deserializer),
        }
    }
}

impl<P, A, AR> LobbyCommandHandler<P, A, AR> for LocalLobbyCommandHandler<P, A, AR>
where
    P: PlayerTrait,
    A: ActivityTrait,
    AR: ActivityResultTrait + Serialize,
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
            LobbyCommand::PlayerInfo {
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
                data,
                ..
            } => {
                let data = (self.activity_data_deserializer)(&data);
                lobby
                    .update_activity_info(&activity_id, data)
                    .ok_or(CommandError::ActivityNotFound(activity_id))?;
                Ok(())
            }
            LobbyCommand::SelectActivity { activity_id } => {
                lobby
                    .select_activity(&activity_id)
                    .ok_or(CommandError::ActivityNotFound(activity_id))?;
                Ok(())
            }
            LobbyCommand::RemovePlayer { participant_id } => {
                lobby
                    .remove_participant(participant_id)
                    .ok_or(CommandError::PlayerNotFound(participant_id))?;
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
            LobbyCommand::AddActivityResult {
                activity_id,
                player_id,
                data,
            } => {
                let data = (self.activity_result_data_deserializer)(&data);
                let activity_result: ActivityResult<AR> =
                    ActivityResult::new(activity_id, player_id, data);
                lobby.add_activity_result(activity_result);
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
            LobbyCommand::RequestState => Ok(()),
        }
    }

    fn send_command(&self, _command: LobbyCommand) -> Result<(), CommandError> {
        // Implementation needed when you have to send the command to a remote server
        Ok(())
    }
}
