use crate::handler::LocalLobbyCommandHandler;
use crate::model::{
    ActivityResultTrait, ActivityTrait, CommandError, Lobby, LobbyCommand, LobbyCommandHandler,
    LobbyCommandWrapper, Player, PlayerTrait,
};
use futures::{SinkExt, StreamExt};
use gloo::net::websocket::{futures::WebSocket, Message};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;
use yew::Callback;
use yew::UseStateHandle;

type WebSocketSender = futures::stream::SplitSink<WebSocket, Message>;

#[derive(Clone)]
pub struct WebSocketLobbyCommandHandler<
    P: PlayerTrait,
    A: ActivityTrait,
    AR: ActivityResultTrait + Serialize,
> {
    lobby_id: Uuid,
    player: UseStateHandle<RefCell<Player<P>>>,
    password: Option<String>,
    local_handler: LocalLobbyCommandHandler<P, A, AR>,
    websocket_url: String,
    lobby: UseStateHandle<RefCell<Lobby<P, A, AR>>>,
    sender: Rc<RefCell<Option<WebSocketSender>>>,
    update_ui: Callback<Lobby<P, A, AR>>,
}

impl<P, A, AR> WebSocketLobbyCommandHandler<P, A, AR>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static + std::fmt::Debug,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static + std::fmt::Debug,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static + std::fmt::Debug,
{
    pub fn new(
        websocket_url: &str,
        lobby_id: Uuid,
        player: UseStateHandle<RefCell<Player<P>>>,
        password: Option<String>,
        local_handler: LocalLobbyCommandHandler<P, A, AR>,
        lobby: UseStateHandle<RefCell<Lobby<P, A, AR>>>,
        update_ui: Callback<Lobby<P, A, AR>>,
    ) -> Self {
        let handler = Self {
            lobby_id,
            player,
            password,
            local_handler,
            websocket_url: websocket_url.to_string(),
            lobby,
            sender: Rc::new(RefCell::new(None)),
            update_ui,
        };
        handler.connect();
        handler
    }

    fn join_lobby(&self) {
        let sender = self.sender.clone();
        let lobby_id = self.lobby_id;
        let player = self.player.borrow().clone();

        let join_command = LobbyCommandWrapper {
            lobby_id,
            password: self.password.clone(),
            command: LobbyCommand::Join {
                player_id: player.id,
                role: player.role,
                lobby_id,
                data: serde_json::to_string(&player.data).unwrap(),
                password: None,
            },
        };

        spawn_local(async move {
            if let Some(write) = sender.borrow_mut().as_mut() {
                let init_message = serde_json::to_string(&join_command).unwrap();
                write
                    .send(Message::Text(init_message))
                    .await
                    .expect("Failed to send lobby ID");
            }
        });
    }

    fn connect(&self) {
        let ws = WebSocket::open(&self.websocket_url).expect("Failed to connect to WebSocket");
        let (write, mut read) = ws.split();

        // Store the sender
        *self.sender.borrow_mut() = Some(write);

        // Handle incoming messages
        let handler = self.clone();
        spawn_local(async move {
            while let Some(msg) = read.next().await {
                if let Ok(Message::Text(text)) = msg {
                    log::debug!("Received message: {:?}", text);
                    if let Ok(command_wrapper) = serde_json::from_str::<LobbyCommandWrapper>(&text)
                    {
                        handler.handle_incoming_message(command_wrapper);
                    }
                }
            }
            log::info!("WebSocket connection closed");
        });
    }

    fn handle_incoming_message(&self, command_wrapper: LobbyCommandWrapper) {
        match command_wrapper.command {
            LobbyCommand::UpdatePlayerId { player_id } => {
                let mut lobby_borrow = self.lobby.borrow_mut();
                self.player.borrow_mut().id = player_id;
                if let Err(e) = self
                    .local_handler
                    .handle_command(&mut *lobby_borrow, command_wrapper.command)
                {
                    log::error!("Error handling command: {:?}", e);
                } else {
                    self.update_ui.emit((*lobby_borrow).clone());
                }

                self.join_lobby();
                log::info!("Player ID updated: {}", player_id);
            }
            LobbyCommand::Join { .. } => {
                if self.lobby.borrow().is_admin() {
                    self.send_lobby_state();
                }
                let mut lobby_borrow = self.lobby.borrow_mut();
                if let Err(e) = self
                    .local_handler
                    .handle_command(&mut *lobby_borrow, command_wrapper.command)
                {
                    log::error!("Error handling command: {:?}", e);
                } else {
                    self.update_ui.emit((*lobby_borrow).clone());
                }
            }
            _ => {
                let mut lobby_borrow = self.lobby.borrow_mut();
                if let Err(e) = self
                    .local_handler
                    .handle_command(&mut *lobby_borrow, command_wrapper.command)
                {
                    log::error!("Error handling command: {:?}", e);
                } else {
                    self.update_ui.emit((*lobby_borrow).clone());
                }
            }
        }
    }

    fn send_lobby_state(&self) {
        log::info!("Sending lobby state");
        for participant in self.lobby.borrow().participants.iter() {
            let command = LobbyCommand::PlayerInfo {
                player_id: participant.id,
                role: participant.role,
                data: serde_json::to_string(&participant.data).unwrap(),
            };
            self.send_command(command).unwrap();
        }
        for activity in self.lobby.borrow().activities.iter() {
            let command = LobbyCommand::ActivityInfo {
                activity_id: activity.id.clone(),
                data: serde_json::to_string(&activity.data).unwrap(),
                status: activity.status.clone(),
            };
            self.send_command(command).unwrap();
        }
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

        let sender = self.sender.clone();
        spawn_local(async move {
            if let Some(write) = sender.borrow_mut().as_mut() {
                if let Err(e) = write.send(Message::Text(message)).await {
                    log::error!("Failed to send message: {:?}", e);
                }
            } else {
                log::error!("WebSocket connection not available");
            }
        });

        Ok(())
    }

    fn reconnect(&self) {
        self.connect();
    }
}

impl<P, A, AR> LobbyCommandHandler<P, A, AR> for WebSocketLobbyCommandHandler<P, A, AR>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static + std::fmt::Debug,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static + std::fmt::Debug,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static + std::fmt::Debug,
{
    fn handle_command(
        &self,
        _lobby: &mut Lobby<P, A, AR>,
        command: LobbyCommand,
    ) -> Result<(), CommandError> {
        // If the sender is None, try to reconnect
        if self.sender.borrow().is_none() {
            self.reconnect();
        }
        self.send_command(command)
    }

    fn send_command(&self, command: LobbyCommand) -> Result<(), CommandError> {
        if self.sender.borrow().is_none() {
            self.reconnect();
        }
        self.send_command(command)
    }
}
