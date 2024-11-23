use super::{error::RepositoryError, Connection, ConnectionRepository, LobbyRepository};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct MemoryStorage {
    connections: Arc<RwLock<HashMap<Uuid, Connection>>>,
    lobbies: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        MemoryStorage {
            connections: Arc::new(RwLock::new(HashMap::new())),
            lobbies: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ConnectionRepository for MemoryStorage {
    async fn add_connection(&self, connection: Connection) -> Result<(), RepositoryError> {
        let mut connections = self.connections.write().await;
        connections.insert(connection.player_id, connection);
        Ok(())
    }

    async fn remove_connection(&self, player_id: Uuid) -> Result<(), RepositoryError> {
        let mut connections = self.connections.write().await;
        if connections.remove(&player_id).is_none() {
            return Err(RepositoryError::NotFound(player_id.to_string()));
        }
        Ok(())
    }

    async fn get_connection(&self, player_id: Uuid) -> Result<Option<Connection>, RepositoryError> {
        let connections = self.connections.read().await;
        Ok(connections.get(&player_id).cloned())
    }

    async fn get_all_connections(&self) -> Result<Vec<Connection>, RepositoryError> {
        let connections = self.connections.read().await;
        Ok(connections.values().cloned().collect())
    }
}

#[async_trait]
impl LobbyRepository for MemoryStorage {
    async fn add_player_to_lobby(
        &self,
        lobby_id: Uuid,
        player_id: Uuid,
    ) -> Result<(), RepositoryError> {
        let mut lobbies = self.lobbies.write().await;
        lobbies
            .entry(lobby_id)
            .or_insert_with(Vec::new)
            .push(player_id);
        Ok(())
    }

    async fn remove_player_from_lobby(
        &self,
        lobby_id: Uuid,
        player_id: Uuid,
    ) -> Result<(), RepositoryError> {
        let mut lobbies = self.lobbies.write().await;
        if let Some(players) = lobbies.get_mut(&lobby_id) {
            players.retain(|&id| id != player_id);
            if players.is_empty() {
                lobbies.remove(&lobby_id);
            }
        } else {
            return Err(RepositoryError::NotFound(lobby_id.to_string()));
        }
        Ok(())
    }

    async fn get_players_in_lobby(&self, lobby_id: Uuid) -> Result<Vec<Uuid>, RepositoryError> {
        let lobbies = self.lobbies.read().await;
        Ok(lobbies.get(&lobby_id).cloned().unwrap_or_default())
    }

    async fn get_all_lobbies(&self) -> Result<Vec<(Uuid, Vec<Uuid>)>, RepositoryError> {
        let lobbies = self.lobbies.read().await;
        Ok(lobbies
            .iter()
            .map(|(&id, players)| (id, players.clone()))
            .collect())
    }
}
