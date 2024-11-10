use crate::model::{LobbyCommandWrapper, Role};
use crate::server::{websocket_server::WebSocketServer, Connection};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct WebSocketListener {}

impl WebSocketListener {
    pub async fn handle_websocket(
        ws: WebSocketUpgrade,
        lobby_id: Option<Uuid>,
        server: WebSocketServer,
    ) -> impl IntoResponse {
        ws.on_upgrade(move |socket| {
            WebSocketListener::websocket_connection(socket, lobby_id, server)
        })
    }

    async fn websocket_connection(
        socket: WebSocket,
        lobby_id: Option<Uuid>,
        server: WebSocketServer,
    ) {
        let (mut ws_sender, mut ws_receiver) = socket.split();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Generate unique player ID
        let player_id = Uuid::new_v4();
        let lobby_id = lobby_id.unwrap_or_else(Uuid::new_v4);
        let role = Role::Participant; // Assume role; this could be set dynamically

        // Create a new Connection and add it to the WebSocketServer
        let connection = Connection {
            sender: tx.clone(),
            player_id,
            lobby_id,
            role,
        };

        server.add_connection(connection).await;

        // Spawn a task to handle outgoing messages to the client
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = ws_sender.send(msg).await {
                    log::error!("Failed to send message: {:?}", e);
                    break;
                }
            }
        });

        // Handle incoming messages from the client
        while let Some(Ok(Message::Text(text))) = ws_receiver.next().await {
            match serde_json::from_str::<LobbyCommandWrapper>(&text) {
                Ok(command_wrapper) => {
                    server.handle_command(&command_wrapper).await;

                    server
                        .broadcast_to_lobby(command_wrapper.lobby_id, &command_wrapper)
                        .await;
                }
                Err(e) => log::error!("Failed to parse command: {:?}", e),
            }
        }

        // Clean up connection on disconnect
        server.remove_connection(lobby_id, player_id).await;
    }
}
