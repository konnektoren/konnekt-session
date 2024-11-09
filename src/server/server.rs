use crate::model::{LobbyCommand, LobbyCommandWrapper, Role};
use crate::server::Connection;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::debug;
use uuid::Uuid;

pub struct WebSocketServer {
    connections: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, Connection>>>>, // lobby_id -> (player_id -> Connection)
}

impl WebSocketServer {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(&self, addr: SocketAddr) {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        let connections = Arc::clone(&self.connections);

        loop {
            let (stream, peer) = listener.accept().await.unwrap();
            debug!("New connection from {:?}", peer);

            let connections = Arc::clone(&connections);

            tokio::spawn(async move {
                WebSocketServer::handle_connection(stream, connections).await;
            });
        }
    }

    async fn handle_connection(
        stream: TcpStream,
        connections: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, Connection>>>>,
    ) {
        let ws_stream = accept_async(stream).await.unwrap();
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Wait for Join command
        if let Some(Ok(Message::Text(text))) = ws_receiver.next().await {
            if let Ok(command_wrapper) = serde_json::from_str::<LobbyCommandWrapper>(&text) {
                if let LobbyCommand::Join {
                    player_id,
                    role,
                    ref data,
                    ref password,
                } = command_wrapper.command
                {
                    let lobby_id = command_wrapper.lobby_id;

                    // Create new connection
                    let connection = Connection {
                        sender: tx.clone(),
                        player_id,
                        lobby_id,
                        role,
                    };

                    // Add connection to lobby
                    {
                        let mut connections_write = connections.write().await;
                        let lobby_connections = connections_write
                            .entry(lobby_id)
                            .or_insert_with(HashMap::new);
                        lobby_connections.insert(player_id, connection);
                    }

                    // Broadcast join notification
                    broadcast_command(Arc::clone(&connections), &command_wrapper, lobby_id).await;

                    // Handle outgoing messages
                    let mut send_task = tokio::spawn(async move {
                        while let Some(message) = rx.recv().await {
                            if let Err(e) = ws_sender.send(message).await {
                                debug!("Error sending message: {}", e);
                                break;
                            }
                        }
                    });

                    // Handle incoming messages
                    let connections_clone = Arc::clone(&connections);
                    let mut recv_task = tokio::spawn(async move {
                        while let Some(Ok(message)) = ws_receiver.next().await {
                            match message {
                                Message::Text(text) => {
                                    if let Ok(command_wrapper) =
                                        serde_json::from_str::<LobbyCommandWrapper>(&text)
                                    {
                                        broadcast_command(
                                            Arc::clone(&connections_clone),
                                            &command_wrapper,
                                            lobby_id,
                                        )
                                        .await;
                                    }
                                }
                                Message::Close(_) => break,
                                _ => {}
                            }
                        }
                        // Remove connection on disconnect
                        let mut connections_write = connections_clone.write().await;
                        if let Some(lobby_connections) = connections_write.get_mut(&lobby_id) {
                            lobby_connections.remove(&player_id);
                            if lobby_connections.is_empty() {
                                connections_write.remove(&lobby_id);
                            }
                        }
                    });

                    // Wait for either task to complete
                    tokio::select! {
                        _ = (&mut send_task) => recv_task.abort(),
                        _ = (&mut recv_task) => send_task.abort(),
                    }
                }
            }
        }
    }
}

async fn broadcast_command(
    connections: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, Connection>>>>,
    command_wrapper: &LobbyCommandWrapper,
    lobby_id: Uuid,
) {
    let connections = connections.read().await;
    let message = Message::Text(serde_json::to_string(command_wrapper).unwrap());

    if let Some(lobby_connections) = connections.get(&lobby_id) {
        for connection in lobby_connections.values() {
            if let Err(e) = connection.sender.send(message.clone()) {
                debug!(
                    "Error broadcasting message to player {}: {}",
                    connection.player_id, e
                );
            }
        }
    }
}
