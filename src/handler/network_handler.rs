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
pub struct NetworkHandler<P, A, AR>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static,
{
    transport: Arc<RwLock<Box<dyn Transport>>>,
    local_handler: LocalLobbyCommandHandler<P, A, AR>,
    client_id: ClientId,
    lobby_id: LobbyId,
    role: Role,
    ping: Arc<RwLock<(Uuid, u128)>>,
}

impl<P, A, AR> PartialEq for NetworkHandler<P, A, AR>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.client_id == other.client_id && self.lobby_id == other.lobby_id
    }
}

impl<P, A, AR> NetworkHandler<P, A, AR>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static,
{
    pub fn new(
        transport: Box<dyn Transport>,
        local_handler: LocalLobbyCommandHandler<P, A, AR>,
        client_id: ClientId,
        lobby_id: LobbyId,
        role: Role,
    ) -> Self {
        Self {
            transport: Arc::new(RwLock::new(transport)),
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
        log::info!("Connecting to lobby {} as {:?}", self.lobby_id, role);

        // First establish transport connection
        if !self.transport.read().unwrap().is_connected() {
            log::info!("Establishing transport connection");
            self.transport.write().unwrap().connect()?;
        }

        // Send connect command
        let connect_command = NetworkCommand::<String>::Connect {
            client_id: self.client_id,
            lobby_id: self.lobby_id,
        };
        self.send_network_command(connect_command)?;

        // Join lobby
        self.join_lobby(player, lobby, role)?;

        log::info!("Successfully connected to lobby");
        Ok(())
    }

    pub fn disconnect(&self) {
        log::info!("Disconnecting from lobby {}", self.lobby_id);

        // Send disconnect command before closing transport
        let disconnect_command = NetworkCommand::<String>::Disconnect {
            client_id: self.client_id,
            lobby_id: self.lobby_id,
        };

        if let Err(e) = self.send_network_command(disconnect_command) {
            log::error!("Failed to send disconnect command: {:?}", e);
        }

        // Close transport connection
        self.transport.write().unwrap().disconnect();
        log::info!("Disconnected from lobby");
    }

    pub fn is_connected(&self) -> bool {
        self.transport.read().unwrap().is_connected()
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
            NetworkCommand::Connect {
                client_id,
                lobby_id,
            } => {
                log::info!(
                    "Received Connect command from client {} for lobby {}",
                    client_id,
                    lobby_id
                );
                if self.role == Role::Admin {
                    log::info!("Admin received connect request from {}", client_id);
                    // Send current lobby state to the new client
                    self.send_lobby_state(lobby);
                }
                Ok(())
            }
            NetworkCommand::Message { data, client_id } => {
                log::info!("Received Message from client {}", client_id);
                let lobby_command = serde_json::from_str::<LobbyCommandWrapper>(&data)
                    .map_err(|_| NetworkError::InvalidData)?;

                // Handle the command locally first
                let result = self
                    .local_handler
                    .handle_command(lobby, lobby_command.command.clone());

                if let Err(error) = result {
                    log::error!("Failed to handle command: {:?}", error);
                    return Ok(());
                }

                // If admin and not self, broadcast state changes to all clients
                if self.role == Role::Admin && client_id != self.client_id {
                    match lobby_command.command {
                        LobbyCommand::RequestState => {
                            log::info!("Admin sending lobby state to {}", client_id);
                            self.send_lobby_state(lobby);
                        }
                        LobbyCommand::SelectActivity { .. } | LobbyCommand::Join { .. } => {
                            // For state-changing commands, broadcast the new state
                            log::info!("Admin broadcasting updated state after command");
                            self.send_lobby_state(lobby);
                        }
                        _ => {
                            // Forward the message to all clients
                            let forward_command = NetworkCommand::Message {
                                client_id,
                                data: data.clone(),
                            };
                            self.send_network_command(forward_command)?;
                        }
                    }
                }
                Ok(())
            }
            NetworkCommand::Ping { id, client_id } => {
                log::debug!("Received Ping from client {}", client_id);
                let command = NetworkCommand::Pong { id, client_id };
                self.send_network_command(command)
            }
            NetworkCommand::Pong { id, client_id } => {
                log::debug!("Received Pong from client {}", client_id);
                if client_id == self.client_id && self.ping.read().unwrap().0 == id {
                    let ping_time = now() - self.ping.read().unwrap().1;
                    *ping = Some(ping_time as u32);
                }
                Ok(())
            }
            NetworkCommand::Disconnect {
                client_id,
                lobby_id,
            } => {
                log::info!("Client {} disconnected from lobby {}", client_id, lobby_id);
                // Handle disconnect - might want to clean up state if admin
                if self.role == Role::Admin {
                    // Add any cleanup needed
                    log::info!("Admin handling disconnect");
                }
                Ok(())
            }
        }
    }

    pub fn send_network_command(
        &self,
        command: NetworkCommand<String>,
    ) -> Result<(), NetworkError> {
        let message = serde_json::to_string(&command).map_err(|_| NetworkError::InvalidData)?;

        // Send through transport
        self.transport
            .read()
            .unwrap()
            .sender()
            .unbounded_send(message.clone())
            .map_err(|_| NetworkError::SendError)?;

        Ok(())
    }
}

impl<P, A, AR> LobbyCommandHandler<P, A, AR> for NetworkHandler<P, A, AR>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static,
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
