use crate::handler::LocalLobbyCommandHandler;
use crate::model::{
    ActivityData, CommandError, Lobby, LobbyCommand, LobbyCommandHandler, LobbyCommandWrapper,
    PlayerData,
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
pub struct WebSocketLobbyCommandHandler<P: PlayerData, A: ActivityData> {
    lobby_id: Uuid,
    local_handler: LocalLobbyCommandHandler<P>,
    websocket_url: String,
    lobby: UseStateHandle<RefCell<Lobby<P, A>>>,
    sender: Rc<RefCell<Option<WebSocketSender>>>,
    update_ui: Callback<(Lobby<P, A>)>,
}

impl<P, A> WebSocketLobbyCommandHandler<P, A>
where
    P: PlayerData + Serialize + for<'de> Deserialize<'de> + 'static + std::fmt::Debug,
    A: ActivityData + 'static + std::fmt::Debug,
{
    pub fn new(
        websocket_url: &str,
        lobby_id: Uuid,
        local_handler: LocalLobbyCommandHandler<P>,
        lobby: UseStateHandle<RefCell<Lobby<P, A>>>,
        update_ui: Callback<(Lobby<P, A>)>,
    ) -> Self {
        let handler = Self {
            lobby_id,
            local_handler,
            websocket_url: websocket_url.to_string(),
            lobby,
            sender: Rc::new(RefCell::new(None)),
            update_ui,
        };
        handler.connect();
        handler
    }

    fn connect(&self) {
        let ws = WebSocket::open(&self.websocket_url).expect("Failed to connect to WebSocket");
        let (write, mut read) = ws.split();

        // Store the sender
        *self.sender.borrow_mut() = Some(write);

        // Send initial lobby ID message
        let sender = self.sender.clone();
        let lobby_id = self.lobby_id;
        spawn_local(async move {
            if let Some(write) = sender.borrow_mut().as_mut() {
                let init_message = serde_json::to_string(&lobby_id).unwrap();
                write
                    .send(Message::Text(init_message))
                    .await
                    .expect("Failed to send lobby ID");
            }
        });

        // Handle incoming messages
        let handler = self.clone();
        spawn_local(async move {
            while let Some(msg) = read.next().await {
                if let Ok(Message::Text(text)) = msg {
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
        let mut lobby_borrow = self.lobby.borrow_mut();

        if let Err(e) = self
            .local_handler
            .handle_command(&mut *lobby_borrow, command_wrapper.command)
        {
            log::error!("Error handling command: {:?}", e);
        } else {
            self.update_ui.emit((&*lobby_borrow).clone());
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

impl<P, A> LobbyCommandHandler<P, A> for WebSocketLobbyCommandHandler<P, A>
where
    P: PlayerData + Serialize + for<'de> Deserialize<'de> + 'static + std::fmt::Debug,
    A: ActivityData + 'static + std::fmt::Debug,
{
    fn handle_command(
        &self,
        _lobby: &mut Lobby<P, A>,
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
