use crate::model::{ClientId, NetworkCommand, NetworkCommandHandler, NetworkError};
use crate::server::v2::{Connection, ConnectionRepository, LobbyRepository};
use async_trait::async_trait;
use axum::extract::ws::Message;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::Sender;
use tracing::{debug, info, instrument};

#[derive(Clone)]
pub struct ConnectionHandler {
    client_id: Arc<RwLock<Option<ClientId>>>,
    connection_repo: Arc<dyn ConnectionRepository + Send + Sync>,
    lobby_repo: Arc<dyn LobbyRepository + Send + Sync>,
    sender: Option<Sender<Message>>,
}

impl ConnectionHandler {
    pub fn new(
        connection_repo: Arc<dyn ConnectionRepository>,
        lobby_repo: Arc<dyn LobbyRepository>,
    ) -> Self {
        ConnectionHandler {
            client_id: Arc::new(RwLock::new(None)),
            sender: None,
            connection_repo,
            lobby_repo,
        }
    }

    pub fn new_from(cloneable: &Self) -> Self {
        ConnectionHandler {
            client_id: Arc::new(RwLock::new(None)),
            sender: None,
            connection_repo: cloneable.connection_repo.clone(),
            lobby_repo: cloneable.lobby_repo.clone(),
        }
    }

    pub fn client_id(&self) -> Option<ClientId> {
        self.client_id.read().unwrap().clone()
    }

    pub fn with_sender(&self, sender: Sender<Message>) -> Self {
        let mut new_self = Self::new_from(self);
        new_self.sender = Some(sender);
        new_self
    }

    pub async fn get_connection(&self) -> Result<Option<Connection>, NetworkError> {
        let client_id = self.client_id.read().unwrap().clone();
        match client_id {
            Some(client_id) => self.connection_repo.get_connection(client_id).await,
            _ => Ok(None),
        }
    }

    #[instrument(skip(self))]
    pub async fn disconnect(&self) -> Result<(), NetworkError> {
        let connection = self.get_connection().await?;

        if let Some(connection) = connection {
            let command = NetworkCommand::Disconnect {
                client_id: connection.client_id.clone(),
                lobby_id: connection.lobby_id,
            };

            self.handle_command(command).await
        } else {
            Ok(())
        }
    }

    #[instrument(skip(self))]
    pub async fn broadcast_command(
        &self,
        command: NetworkCommand<String>,
    ) -> Result<(), NetworkError> {
        let lobby_id = self.get_connection().await?.unwrap().lobby_id;
        let connections = self.connection_repo.get_all_connections().await?;

        for connection in connections {
            if connection.lobby_id != lobby_id {
                continue;
            }
            self.send_command_to_client(connection.client_id, command.clone())
                .await?;
        }
        Ok(())
    }
    pub async fn send_command_to_client(
        &self,
        client_id: ClientId,
        command: NetworkCommand<String>,
    ) -> Result<(), NetworkError> {
        let connection = self.connection_repo.get_connection(client_id).await?;

        if let Some(connection) = connection {
            let command = serde_json::to_string(&command).map_err(|_| NetworkError::InvalidData)?;
            connection
                .sender
                .send(Message::Text(command))
                .await
                .map_err(|e| NetworkError::InternalError(e.to_string()))?;
        }
        Ok(())
    }
}

#[async_trait]
impl NetworkCommandHandler<String> for ConnectionHandler {
    #[instrument(skip(self, command))]
    async fn handle_command(&self, command: NetworkCommand<String>) -> Result<(), NetworkError> {
        if let Some(sender) = &self.sender {
            match command {
                NetworkCommand::Connect {
                    client_id,
                    lobby_id,
                } => {
                    info!(?client_id, ?lobby_id, "Processing connection request");
                    let connection = Connection::new(client_id, lobby_id, sender.clone());
                    self.client_id.write().unwrap().replace(client_id);

                    debug!("Adding connection to repository");
                    self.connection_repo.add_connection(connection).await?;

                    debug!("Adding client to lobby");
                    self.lobby_repo
                        .add_client_to_lobby(client_id, lobby_id)
                        .await?;

                    info!("Connection request processed successfully");
                }
                NetworkCommand::Disconnect {
                    client_id,
                    lobby_id,
                } => {
                    self.lobby_repo
                        .remove_client_from_lobby(client_id, lobby_id)
                        .await?;
                    self.connection_repo.remove_connection(client_id).await?;
                }
                NetworkCommand::Message { client_id, data } => {
                    let command = NetworkCommand::Message {
                        client_id,
                        data: data.clone(),
                    };
                    self.send_command(command).await?;
                }
                NetworkCommand::Ping { id, client_id } => {
                    let command = NetworkCommand::Pong { id, client_id };
                    self.send_command(command).await?;
                }
                NetworkCommand::Pong { id, client_id } => {
                    let command = NetworkCommand::Ping { id, client_id };
                    self.send_command(command).await?;
                }
            }
        }
        Ok(())
    }

    #[instrument(skip(self, command), fields(command_type = ?command.get_type()))]
    async fn send_command(&self, command: NetworkCommand<String>) -> Result<(), NetworkError> {
        match command {
            NetworkCommand::Message { client_id, data } => {
                let command = NetworkCommand::Message {
                    client_id,
                    data: data.clone(),
                };
                self.broadcast_command(command).await?;
            }
            NetworkCommand::Ping { id, client_id } => {
                let command: NetworkCommand<String> = NetworkCommand::Ping { id, client_id };
                self.broadcast_command(command).await?;
            }
            NetworkCommand::Pong { id, client_id } => {
                let command: NetworkCommand<String> = NetworkCommand::Pong { id, client_id };
                self.broadcast_command(command).await?;
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ClientId, LobbyId};
    use crate::server::v2::MemoryStorage;

    #[tokio::test]
    async fn test_connect_command() {
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let connection_repo = Arc::new(MemoryStorage::new());
        let lobby_repo = Arc::new(MemoryStorage::new());
        let handler =
            ConnectionHandler::new(connection_repo.clone(), lobby_repo.clone()).with_sender(tx);

        let command = NetworkCommand::Connect {
            client_id: ClientId::nil(),
            lobby_id: LobbyId::nil(),
        };

        let result = handler.handle_command(command).await;
        assert!(result.is_ok());

        let connections = connection_repo.get_all_connections().await.unwrap();
        assert_eq!(connections.len(), 1);

        let lobby = lobby_repo
            .get_clients_in_lobby(LobbyId::nil())
            .await
            .unwrap();
        assert_eq!(lobby.len(), 1);
        assert_eq!(lobby[0], ClientId::nil());
    }

    #[tokio::test]
    async fn test_disconnect_command() {
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let connection_repo = Arc::new(MemoryStorage::new());
        let lobby_repo = Arc::new(MemoryStorage::new());
        let handler =
            ConnectionHandler::new(connection_repo.clone(), lobby_repo.clone()).with_sender(tx);

        let command = NetworkCommand::Connect {
            client_id: ClientId::nil(),
            lobby_id: LobbyId::nil(),
        };

        handler.handle_command(command).await.unwrap();

        let connections = connection_repo.get_all_connections().await.unwrap();
        assert_eq!(connections.len(), 1);

        let command = NetworkCommand::Disconnect {
            client_id: connections[0].client_id.clone(),
            lobby_id: LobbyId::nil(),
        };

        handler.handle_command(command).await.unwrap();

        let connections = connection_repo.get_all_connections().await.unwrap();
        assert_eq!(connections.len(), 0);

        let lobby = lobby_repo
            .get_clients_in_lobby(LobbyId::nil())
            .await
            .unwrap();
        assert_eq!(lobby.len(), 0);
    }

    #[tokio::test]
    async fn test_disconnect() {
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        let connection_repo = Arc::new(MemoryStorage::new());
        let lobby_repo = Arc::new(MemoryStorage::new());
        let handler = ConnectionHandler::new(connection_repo.clone(), lobby_repo.clone());
        let handler = handler.with_sender(tx);

        let command = NetworkCommand::Connect {
            client_id: ClientId::nil(),
            lobby_id: LobbyId::nil(),
        };

        handler.handle_command(command).await.unwrap();

        let connections = connection_repo.get_all_connections().await.unwrap();
        assert_eq!(connections.len(), 1);

        handler.disconnect().await.unwrap();

        let connections = connection_repo.get_all_connections().await.unwrap();
        assert_eq!(connections.len(), 0);
    }
}
