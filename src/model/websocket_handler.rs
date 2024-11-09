use crate::model::{
    ActivityData, CommandError, Lobby, LobbyCommand, LobbyCommandHandler, LobbyCommandWrapper,
    LocalLobbyCommandHandler, PlayerData,
};
use futures::{SinkExt, StreamExt};
use gloo::net::websocket::{futures::WebSocket, Message};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Clone)]
pub struct WebSocketLobbyCommandHandler<P: PlayerData, A: ActivityData> {
    lobby_id: Uuid,
    local_handler: LocalLobbyCommandHandler<P>,
    on_command: Callback<Result<(), CommandError>>,
    websocket_url: String,
    lobby: Arc<Mutex<Lobby<P, A>>>,
}

impl<P, A> WebSocketLobbyCommandHandler<P, A>
where
    P: PlayerData + Serialize + for<'de> Deserialize<'de> + 'static,
    A: ActivityData + 'static,
{
    pub fn new(
        websocket_url: &str,
        lobby_id: Uuid,
        local_handler: LocalLobbyCommandHandler<P>,
        on_command: Callback<Result<(), CommandError>>,
        lobby: Arc<Mutex<Lobby<P, A>>>,
    ) -> Self {
        let handler = Self {
            lobby_id,
            local_handler,
            on_command,
            websocket_url: websocket_url.to_string(),
            lobby,
        };
        handler.connect();
        handler
    }

    fn connect(&self) {
        let ws = WebSocket::open(&self.websocket_url).expect("Failed to connect to WebSocket");
        let (mut write, mut read) = ws.split();
        let lobby_id = self.lobby_id;
        let on_command = self.on_command.clone();
        let local_handler = self.local_handler.clone();

        // Send initial lobby ID message
        let init_message = serde_json::to_string(&lobby_id).unwrap();
        spawn_local(async move {
            write
                .send(Message::Text(init_message))
                .await
                .expect("Failed to send lobby ID");
        });

        let lobby = self.lobby.clone();
        // Handle incoming messages
        spawn_local(async move {
            let lobby = lobby.clone();
            while let Some(msg) = read.next().await {
                if let Ok(Message::Text(text)) = msg {
                    if let Ok(command_wrapper) = serde_json::from_str::<LobbyCommandWrapper>(&text)
                    {
                        let mut lobby = lobby.lock().unwrap();
                        on_command.emit(
                            local_handler.handle_command(&mut lobby, command_wrapper.command),
                        );
                    }
                }
            }
            log::debug!("WebSocket connection closed");
        });
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

        let mut ws = WebSocket::open(&self.websocket_url).map_err(|_| {
            CommandError::InvalidOperation("Failed to connect to WebSocket".to_string())
        })?;

        spawn_local(async move {
            if let Err(e) = ws.send(Message::Text(message)).await {
                log::error!("Failed to send message: {:?}", e);
            }
        });

        Ok(())
    }
}

impl<P, A> LobbyCommandHandler<P, A> for WebSocketLobbyCommandHandler<P, A>
where
    P: PlayerData + Serialize + for<'de> Deserialize<'de> + 'static,
    A: ActivityData + 'static,
{
    fn handle_command(
        &self,
        lobby: &mut Lobby<P, A>,
        command: LobbyCommand,
    ) -> Result<(), CommandError> {
        self.send_command(command)
    }
}
