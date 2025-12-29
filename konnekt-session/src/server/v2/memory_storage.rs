use super::{Connection, ConnectionRepository, LobbyRepository};
use crate::model::{ClientId, LobbyId, NetworkError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, error, instrument};

pub struct MemoryStorage {
    connections: Arc<RwLock<HashMap<ClientId, Connection>>>,
    lobbies_to_clients: Arc<RwLock<HashMap<LobbyId, Vec<ClientId>>>>,
    clients_to_lobbies: Arc<RwLock<HashMap<ClientId, LobbyId>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            lobbies_to_clients: Arc::new(RwLock::new(HashMap::new())),
            clients_to_lobbies: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ConnectionRepository for MemoryStorage {
    #[instrument(skip(self, connection))]
    async fn add_connection(&self, connection: Connection) -> Result<(), NetworkError> {
        debug!(?connection.client_id, ?connection.lobby_id, "Adding connection");
        match self.connections.write() {
            Ok(mut connections) => {
                connections.insert(connection.client_id.clone(), connection);
                debug!("Connection added successfully");
                Ok(())
            }
            Err(e) => {
                error!(?e, "Failed to add connection");
                Err(NetworkError::InternalError(e.to_string()))
            }
        }
    }

    #[instrument(skip(self))]
    async fn remove_connection(&self, id: ClientId) -> Result<(), NetworkError> {
        debug!(?id, "Removing connection");
        match self.connections.write() {
            Ok(mut connections) => {
                connections.remove(&id);
                debug!("Connection removed successfully");
                Ok(())
            }
            Err(e) => {
                error!(?e, "Failed to remove connection");
                Err(NetworkError::InternalError(e.to_string()))
            }
        }
    }

    #[instrument(skip(self))]
    async fn get_connection(&self, id: ClientId) -> Result<Option<Connection>, NetworkError> {
        debug!(?id, "Getting connection");
        match self.connections.read() {
            Ok(connections) => {
                let connection = connections.get(&id).cloned();
                debug!(?connection, "Connection retrieval completed");
                Ok(connection)
            }
            Err(e) => {
                error!(?e, "Failed to get connection");
                Err(NetworkError::InternalError(e.to_string()))
            }
        }
    }

    #[instrument(skip(self))]
    async fn get_all_connections(&self) -> Result<Vec<Connection>, NetworkError> {
        debug!("Getting all connections");
        match self.connections.read() {
            Ok(connections) => {
                let connections: Vec<_> = connections.values().cloned().collect();
                debug!(
                    connection_count = connections.len(),
                    "Retrieved all connections"
                );
                Ok(connections)
            }
            Err(e) => {
                error!(?e, "Failed to get all connections");
                Err(NetworkError::InternalError(e.to_string()))
            }
        }
    }
}

#[async_trait]
impl LobbyRepository for MemoryStorage {
    #[instrument(skip(self))]
    async fn add_client_to_lobby(
        &self,
        lobby_id: LobbyId,
        client_id: ClientId,
    ) -> Result<(), NetworkError> {
        debug!(?lobby_id, ?client_id, "Adding client to lobby");
        match self.lobbies_to_clients.write() {
            Ok(mut lobbies) => {
                lobbies
                    .entry(lobby_id)
                    .or_insert_with(Vec::new)
                    .push(client_id);
                debug!("Client added to lobby successfully");
                Ok(())
            }
            Err(e) => {
                error!(?e, "Failed to add client to lobby");
                Err(NetworkError::InternalError(e.to_string()))
            }
        }?;

        match self.clients_to_lobbies.write() {
            Ok(mut clients) => {
                clients.insert(client_id, lobby_id);
                debug!("Client-lobby mapping updated");
                Ok(())
            }
            Err(e) => {
                error!(?e, "Failed to update client-lobby mapping");
                Err(NetworkError::InternalError(e.to_string()))
            }
        }
    }

    #[instrument(skip(self))]
    async fn remove_client_from_lobby(
        &self,
        lobby_id: LobbyId,
        client_id: ClientId,
    ) -> Result<(), NetworkError> {
        debug!(?lobby_id, ?client_id, "Removing client from lobby");
        match self.lobbies_to_clients.write() {
            Ok(mut lobbies) => {
                if let Some(clients) = lobbies.get_mut(&lobby_id) {
                    clients.retain(|&id| id != client_id);
                }
                Ok(())
            }
            Err(e) => Err(NetworkError::InternalError(e.to_string())),
        }?;
        match self.clients_to_lobbies.write() {
            Ok(mut clients) => {
                clients.remove(&client_id);
                Ok(())
            }
            Err(e) => Err(NetworkError::InternalError(e.to_string())),
        }
    }

    #[instrument(skip(self))]
    async fn get_clients_in_lobby(&self, lobby_id: LobbyId) -> Result<Vec<ClientId>, NetworkError> {
        debug!(?lobby_id, "Getting clients in lobby");
        match self.lobbies_to_clients.read() {
            Ok(lobbies) => Ok(lobbies.get(&lobby_id).cloned().unwrap_or_default()),
            Err(e) => Err(NetworkError::InternalError(e.to_string())),
        }
    }

    #[instrument(skip(self))]
    async fn get_lobby_with_client(
        &self,
        client_id: ClientId,
    ) -> Result<Option<LobbyId>, NetworkError> {
        debug!(?client_id, "Getting lobby for client");
        match self.clients_to_lobbies.read() {
            Ok(clients) => Ok(clients.get(&client_id).cloned()),
            Err(e) => Err(NetworkError::InternalError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LobbyId;

    #[tokio::test]
    async fn test_add_connection() {
        let storage = MemoryStorage::new();
        let connection = Connection::new(
            ClientId::new_v4(),
            LobbyId::new_v4(),
            tokio::sync::mpsc::channel(1).0,
        );
        let result = storage.add_connection(connection.clone()).await;
        assert!(result.is_ok());
        assert_eq!(
            storage
                .get_connection(connection.client_id.clone())
                .await
                .unwrap(),
            Some(connection)
        );
    }

    #[tokio::test]
    async fn test_remove_connection() {
        let storage = MemoryStorage::new();
        let connection = Connection::new(
            ClientId::new_v4(),
            LobbyId::new_v4(),
            tokio::sync::mpsc::channel(1).0,
        );
        storage.add_connection(connection.clone()).await.unwrap();
        let result = storage
            .remove_connection(connection.client_id.clone())
            .await;
        assert!(result.is_ok());
        assert_eq!(
            storage
                .get_connection(connection.client_id.clone())
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn test_get_connection() {
        let storage = MemoryStorage::new();
        let connection = Connection::new(
            ClientId::new_v4(),
            LobbyId::new_v4(),
            tokio::sync::mpsc::channel(1).0,
        );
        storage.add_connection(connection.clone()).await.unwrap();
        assert_eq!(
            storage
                .get_connection(connection.client_id.clone())
                .await
                .unwrap(),
            Some(connection)
        );
    }

    #[tokio::test]
    async fn test_get_all_connections() {
        let storage = MemoryStorage::new();
        let connection1 = Connection::new(
            ClientId::new_v4(),
            LobbyId::new_v4(),
            tokio::sync::mpsc::channel(1).0,
        );
        let connection2 = Connection::new(
            ClientId::new_v4(),
            LobbyId::new_v4(),
            tokio::sync::mpsc::channel(1).0,
        );
        storage.add_connection(connection1.clone()).await.unwrap();
        storage.add_connection(connection2.clone()).await.unwrap();

        assert!(storage.get_all_connections().await.is_ok());
        assert_eq!(storage.get_all_connections().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_add_client_to_lobby() {
        let storage = MemoryStorage::new();
        let client_id = ClientId::new_v4();
        let lobby_id = LobbyId::new_v4();
        let result = storage.add_client_to_lobby(lobby_id, client_id).await;
        assert!(result.is_ok());
        assert_eq!(
            storage.get_clients_in_lobby(lobby_id).await.unwrap(),
            vec![client_id]
        );
    }

    #[tokio::test]
    async fn test_remove_client_from_lobby() {
        let storage = MemoryStorage::new();
        let client_id = ClientId::new_v4();
        let lobby_id = LobbyId::new_v4();
        storage
            .add_client_to_lobby(lobby_id, client_id)
            .await
            .unwrap();
        let result = storage.remove_client_from_lobby(lobby_id, client_id).await;
        assert!(result.is_ok());
        assert_eq!(
            storage.get_clients_in_lobby(lobby_id).await.unwrap(),
            vec![]
        );
    }

    #[tokio::test]
    async fn test_get_clients_in_lobby() {
        let storage = MemoryStorage::new();
        let client_id1 = ClientId::from_u128(1);
        let client_id2 = ClientId::from_u128(2);
        let lobby_id = LobbyId::new_v4();
        storage
            .add_client_to_lobby(lobby_id, client_id1)
            .await
            .unwrap();
        storage
            .add_client_to_lobby(lobby_id, client_id2)
            .await
            .unwrap();

        let clients = storage.get_clients_in_lobby(lobby_id).await.unwrap();
        assert_eq!(clients.len(), 2);

        storage
            .remove_client_from_lobby(lobby_id, client_id1)
            .await
            .unwrap();

        let lobby = storage.get_lobby_with_client(client_id1).await.unwrap();
        assert_eq!(lobby, None);

        let lobby = storage.get_lobby_with_client(client_id2).await.unwrap();
        assert_eq!(lobby, Some(lobby_id));
    }
}
