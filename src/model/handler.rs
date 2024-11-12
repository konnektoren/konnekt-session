use serde::Serialize;

use super::{ActivityData, ActivityResultTrait, CommandError, Lobby, LobbyCommand, PlayerTrait};

pub trait LobbyCommandHandler<P, A, AR>
where
    P: PlayerTrait,
    A: ActivityData,
    AR: ActivityResultTrait + Serialize,
{
    fn handle_command(
        &self,
        lobby: &mut Lobby<P, A, AR>,
        command: LobbyCommand,
    ) -> Result<(), CommandError>;

    fn send_command(&self, command: LobbyCommand) -> Result<(), CommandError>;
}
