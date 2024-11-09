use crate::model::{LobbyCommandWrapper, Role};
use crate::server::{websocket_server::WebSocketServer, Connection};
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;

pub struct WebSocketListener {
    server: WebSocketServer,
    address: SocketAddr,
}

impl WebSocketListener {
    pub fn new(server: WebSocketServer, address: SocketAddr) -> Self {
        WebSocketListener { server, address }
    }

    pub async fn run(&self) {
        // Bind TCP listener to the specified address
        let listener = TcpListener::bind(&self.address)
            .await
            .expect("Failed to bind to address");

        log::info!("WebSocket server listening on ws://{}", self.address);

        // Accept incoming connections
        while let Ok((stream, _)) = listener.accept().await {
            let server = self.server.clone();

            // Spawn a new task to handle each connection
            tokio::spawn(async move {
                match accept_async(stream).await {
                    Ok(ws_stream) => {
                        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
                        let (tx, mut rx) = mpsc::unbounded_channel();

                        // Generate unique player and lobby IDs (in practice, lobby ID would be sent by client)
                        let player_id = Uuid::new_v4();
                        let lobby_id = Uuid::new_v4(); // Or received from client message
                        let role = Role::Participant; // Assume role; this could be set dynamically

                        // Create a new Connection and add it to the WebSocketServer
                        let connection = Connection {
                            sender: tx.clone(),
                            player_id,
                            lobby_id,
                            role,
                        };

                        server.add_connection(lobby_id, player_id, connection).await;

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
                                    server
                                        .broadcast_to_lobby(
                                            command_wrapper.lobby_id,
                                            &command_wrapper,
                                        )
                                        .await;
                                }
                                Err(e) => log::error!("Failed to parse command: {:?}", e),
                            }
                        }

                        // Clean up connection on disconnect
                        server.remove_connection(lobby_id, player_id).await;
                    }
                    Err(e) => log::error!("WebSocket handshake failed: {:?}", e),
                }
            });
        }
    }
}
