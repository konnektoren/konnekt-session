use super::{ActivityData, CommandError, Lobby, LobbyCommand, PlayerData};

pub trait LobbyCommandHandler<P, A>
where
    P: PlayerData,
    A: ActivityData,
{
    fn handle_command(
        &self,
        lobby: &mut Lobby<P, A>,
        command: LobbyCommand,
    ) -> Result<(), CommandError>;

    fn send_command(&self, command: LobbyCommand) -> Result<(), CommandError>;
}
