use super::{ActivityData, ActivityResultData, CommandError, Lobby, LobbyCommand, PlayerData};

pub trait LobbyCommandHandler<P, A, AR>
where
    P: PlayerData,
    A: ActivityData,
    AR: ActivityResultData,
{
    fn handle_command(
        &self,
        lobby: &mut Lobby<P, A, AR>,
        command: LobbyCommand,
    ) -> Result<(), CommandError>;

    fn send_command(&self, command: LobbyCommand) -> Result<(), CommandError>;
}
