use serde::Serialize;

use super::{ActivityResultTrait, ActivityTrait, CommandError, Lobby, LobbyCommand, PlayerTrait};

pub trait LobbyCommandHandler<P, A, AR>
where
    P: PlayerTrait,
    A: ActivityTrait,
    AR: ActivityResultTrait + Serialize,
{
    fn handle_command(
        &self,
        lobby: &mut Lobby<P, A, AR>,
        command: LobbyCommand,
    ) -> Result<(), CommandError>;

    fn send_command(&self, command: LobbyCommand) -> Result<(), CommandError>;
}
