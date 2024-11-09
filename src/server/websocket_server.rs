use crate::model::LobbyCommandWrapper;
use crate::server::Connection;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

#[derive(Clone)]
pub struct WebSocketServer {
    connections: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, Connection>>>>, // lobby_id -> (player_id -> Connection)
}

impl WebSocketServer {
    pub fn new() -> Self {
        WebSocketServer {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_connection(&self, lobby_id: Uuid, player_id: Uuid, connection: Connection) {
        let mut connections = self.connections.write().await;
        connections
            .entry(lobby_id)
            .or_insert_with(HashMap::new)
            .insert(player_id, connection);
    }

    pub async fn broadcast_to_lobby(&self, lobby_id: Uuid, command: &LobbyCommandWrapper) {
        log::debug!("Broadcasting command to lobby {}: {:?}", lobby_id, command);
        log::debug!("map {:?}", self.connections);

        let connections = self.connections.read().await;
        if let Some(lobby) = connections.get(&lobby_id) {
            let serialized_message = match serde_json::to_string(command) {
                Ok(msg) => msg,
                Err(e) => {
                    log::error!("Failed to serialize message: {:?}", e);
                    return;
                }
            };

            log::debug!(
                "Broadcasting message to lobby {}: {}",
                lobby_id,
                serialized_message
            );

            for (_, connection) in lobby {
                if let Err(e) = connection
                    .sender
                    .send(Message::Text(serialized_message.clone()))
                {
                    log::error!(
                        "Failed to send message to player {}: {:?}",
                        connection.player_id,
                        e
                    );
                }
            }
        }
    }

    pub async fn remove_connection(&self, lobby_id: Uuid, player_id: Uuid) {
        let mut connections = self.connections.write().await;
        if let Some(lobby) = connections.get_mut(&lobby_id) {
            lobby.remove(&player_id);
            if lobby.is_empty() {
                connections.remove(&lobby_id);
            }
        }
    }
}
