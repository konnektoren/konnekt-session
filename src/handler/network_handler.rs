use crate::handler::LocalLobbyCommandHandler;
use crate::model::{
    ActivityResultTrait, ActivityTrait, ClientId, CommandError, Lobby, LobbyCommand,
    LobbyCommandHandler, LobbyCommandWrapper, LobbyId, NetworkCommand, NetworkError, Player,
    PlayerTrait, Role, Transport,
};
use instant::SystemTime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::RwLock;
use uuid::Uuid;

fn now() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

#[derive(Clone)]
pub struct NetworkHandler<P, A, AR, T>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    T: Transport + Clone + 'static,
{
    transport: T,
    local_handler: LocalLobbyCommandHandler<P, A, AR>,
    client_id: ClientId,
    lobby_id: LobbyId,
    role: Role,
    ping: Arc<RwLock<(Uuid, u128)>>,
}

impl<P, A, AR, T> PartialEq for NetworkHandler<P, A, AR, T>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    T: Transport + Clone + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.client_id == other.client_id && self.lobby_id == other.lobby_id
    }
}

impl<P, A, AR, T> NetworkHandler<P, A, AR, T>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    T: Transport + Clone + 'static,
{
    pub fn new(
        transport: T,
        local_handler: LocalLobbyCommandHandler<P, A, AR>,
        client_id: ClientId,
        lobby_id: LobbyId,
        role: Role,
    ) -> Self {
        Self {
            transport,
            local_handler,
            client_id,
            lobby_id,
            role,
            ping: Arc::new(RwLock::new((Uuid::new_v4(), now()))),
        }
    }

    pub fn set_role(&mut self, role: Role) {
        self.role = role;
    }

    pub fn connect(
        &self,
        player: &Player<P>,
        lobby: &Lobby<P, A, AR>,
        role: Role,
    ) -> Result<(), NetworkError> {
        let connect_command = NetworkCommand::<String>::Connect {
            client_id: self.client_id,
            lobby_id: self.lobby_id,
        };
        self.send_network_command(connect_command)?;
        self.join_lobby(player, lobby, role)?;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.transport.disconnect();
    }

    pub fn is_connected(&self) -> bool {
        self.transport.is_connected()
    }

    fn join_lobby(
        &self,
        player: &Player<P>,
        lobby: &Lobby<P, A, AR>,
        role: Role,
    ) -> Result<(), NetworkError> {
        let data = serde_json::to_string(&player.data).map_err(|_| NetworkError::InvalidData)?;

        let join_command = LobbyCommand::Join {
            player_id: lobby.player_id,
            lobby_id: lobby.id,
            role,
            data,
            password: None,
        };
        self.send_command(join_command)
            .map_err(|_| NetworkError::InvalidData)?;

        let request_state_command = LobbyCommand::RequestState;
        self.send_command(request_state_command)
            .map_err(|_| NetworkError::InvalidData)?;
        Ok(())
    }

    fn send_lobby_state(&self, lobby: &Lobby<P, A, AR>) {
        log::info!("Sending lobby state");
        for participant in lobby.participants.iter() {
            let command = LobbyCommand::PlayerInfo {
                player_id: participant.id,
                role: participant.role,
                data: serde_json::to_string(&participant.data).unwrap(),
            };
            self.send_command(command).unwrap();
        }
        for activity in lobby.activities.iter() {
            let command = LobbyCommand::ActivityInfo {
                activity_id: activity.id.clone(),
                data: serde_json::to_string(&activity.data).unwrap(),
                status: activity.status.clone(),
            };
            self.send_command(command).unwrap();
        }
    }

    pub fn send_ping(&self) {
        let id = Uuid::new_v4();
        self.ping.write().unwrap().0 = id;
        self.ping.write().unwrap().1 = now();
        let command = NetworkCommand::Ping {
            id,
            client_id: self.client_id,
        };
        self.send_network_command(command).unwrap();
    }

    pub fn handle_message(
        &self,
        lobby: &mut Lobby<P, A, AR>,
        ping: &mut Option<u32>,
        message: String,
    ) -> Result<(), NetworkError> {
        let command = serde_json::from_str::<NetworkCommand<String>>(&message)
            .map_err(|_| NetworkError::InvalidData)?;
        self.handle_network_command(lobby, ping, command)
    }

    pub fn handle_network_command(
        &self,
        lobby: &mut Lobby<P, A, AR>,
        ping: &mut Option<u32>,
        command: NetworkCommand<String>,
    ) -> Result<(), NetworkError> {
        match command {
            NetworkCommand::Message { data, .. } => {
                let lobby_command = serde_json::from_str::<LobbyCommandWrapper>(&data)
                    .map_err(|_| NetworkError::InvalidData)?;

                if let LobbyCommand::RequestState = lobby_command.command {
                    if self.role == Role::Admin {
                        self.send_lobby_state(lobby);
                    }
                }

                let result = self
                    .local_handler
                    .handle_command(lobby, lobby_command.command);

                if let Err(error) = result {
                    log::error!("Failed to handle command: {:?}", error);
                }
            }
            NetworkCommand::Ping { id, client_id } => {
                let command = NetworkCommand::Pong { id, client_id };
                self.send_network_command(command)?;
            }
            NetworkCommand::Pong { id, client_id } => {
                if client_id == self.client_id && self.ping.read().unwrap().0 == id {
                    let ping_time = now() - self.ping.read().unwrap().1;
                    *ping = Some(ping_time as u32);
                }
            }
            _ => {
                log::error!("Unsupported command: {:?}", command);
            }
        }
        Ok(())
    }

    pub fn send_network_command(
        &self,
        command: NetworkCommand<String>,
    ) -> Result<(), NetworkError> {
        let message = serde_json::to_string(&command).map_err(|_| NetworkError::InvalidData)?;
        self.transport
            .sender()
            .unbounded_send(message)
            .map_err(|_| NetworkError::SendError)?;
        Ok(())
    }
}

impl<P, A, AR, T> LobbyCommandHandler<P, A, AR> for NetworkHandler<P, A, AR, T>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    T: Transport + Clone + 'static,
{
    fn handle_command(
        &self,
        lobby: &mut Lobby<P, A, AR>,
        command: LobbyCommand,
    ) -> Result<(), CommandError> {
        self.local_handler.handle_command(lobby, command)?;
        Ok(())
    }

    fn send_command(&self, command: LobbyCommand) -> Result<(), CommandError> {
        let command_wrapper = LobbyCommandWrapper {
            lobby_id: self.lobby_id,
            password: None,
            command,
        };

        let message = serde_json::to_string(&command_wrapper).map_err(|_| {
            CommandError::InvalidOperation("Failed to serialize command".to_string())
        })?;

        let network_command = NetworkCommand::Message {
            client_id: self.client_id,
            data: message,
        };

        self.send_network_command(network_command)
            .map_err(|_| CommandError::InvalidOperation("Failed to send command".to_string()))?;
        Ok(())
    }
}
