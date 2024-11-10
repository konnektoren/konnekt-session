use crate::model::{LobbyCommand, LobbyCommandWrapper};
use crate::server::Connection;
use axum::extract::ws::Message;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct WebSocketServer {
    connections: Arc<RwLock<HashMap<Uuid, Connection>>>, // player_id -> Connection
    lobbies: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,      // lobby_id -> Vec<player_id>
}

impl WebSocketServer {
    pub fn new() -> Self {
        WebSocketServer {
            connections: Arc::new(RwLock::new(HashMap::new())),
            lobbies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_connection(&self, connection: Connection) {
        let player_id: Uuid = connection.player_id.clone();
        let lobby_id: Uuid = connection.lobby_id.clone();

        let mut lobbies = self.lobbies.write().await;
        lobbies
            .entry(lobby_id)
            .or_insert_with(Vec::new)
            .push(player_id.clone());

        log::info!("Player {} joined lobby {}", player_id, lobby_id);

        self.send_command(
            &LobbyCommandWrapper {
                lobby_id,
                password: None,
                command: LobbyCommand::UpdateConnection { player_id },
            },
            &connection,
        )
        .await;
        let mut connections = self.connections.write().await;
        connections.insert(player_id, connection);
    }

    pub async fn handle_command(&self, command: &LobbyCommandWrapper) {
        match command.command {
            LobbyCommand::Join {
                player_id,
                lobby_id,
                ..
            } => {
                let mut lobbies = self.lobbies.write().await;
                lobbies
                    .entry(lobby_id)
                    .or_insert_with(Vec::new)
                    .push(player_id);

                log::info!("Player {} joined lobby {}", player_id, lobby_id);
            }
            _ => {}
        }
    }

    pub async fn broadcast_to_lobby(&self, lobby_id: Uuid, command: &LobbyCommandWrapper) {
        let lobbies = self.lobbies.read().await;
        if let Some(player_ids) = lobbies.get(&lobby_id) {
            let connections = self.connections.read().await;
            for player_id in player_ids {
                log::debug!("Sending message to player {}", player_id);
                if let Some(connection) = connections.get(player_id) {
                    self.send_command(command, connection).await;
                }
            }
        }
    }

    pub async fn send_command(&self, command: &LobbyCommandWrapper, connection: &Connection) {
        let serialized_message = match serde_json::to_string(command) {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("Failed to serialize message: {:?}", e);
                return;
            }
        };

        if let Err(e) = connection.sender.send(Message::Text(serialized_message)) {
            log::error!(
                "Failed to send message to player {}: {:?}",
                connection.player_id,
                e
            );
        }
    }

    pub async fn remove_connection(&self, lobby_id: Uuid, player_id: Uuid) {
        let mut connections = self.connections.write().await;
        connections.remove(&player_id);

        let mut lobbies = self.lobbies.write().await;
        if let Some(player_ids) = lobbies.get_mut(&lobby_id) {
            player_ids.retain(|&id| id != player_id);
            if player_ids.is_empty() {
                lobbies.remove(&lobby_id);
            }
        }
    }
}
