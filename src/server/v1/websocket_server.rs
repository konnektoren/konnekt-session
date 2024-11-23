use crate::model::{LobbyCommand, LobbyCommandWrapper};
use crate::server::v1::{Connection, ConnectionRepository, LobbyRepository};
use axum::extract::ws::Message;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct WebSocketServer {
    connection_repo: Arc<dyn ConnectionRepository>,
    lobby_repo: Arc<dyn LobbyRepository>,
}

impl WebSocketServer {
    pub fn new(
        connection_repo: Arc<dyn ConnectionRepository>,
        lobby_repo: Arc<dyn LobbyRepository>,
    ) -> Self {
        WebSocketServer {
            connection_repo,
            lobby_repo,
        }
    }

    pub async fn add_connection(&self, connection: Connection) {
        let player_id: Uuid = connection.player_id.clone();
        let lobby_id: Uuid = connection.lobby_id.clone();

        if let Err(e) = self
            .lobby_repo
            .add_player_to_lobby(lobby_id, player_id)
            .await
        {
            log::error!("Failed to add player to lobby: {:?}", e);
            return;
        }
        log::info!("Player {} joined lobby {}", player_id, lobby_id);

        self.send_command(
            &LobbyCommandWrapper {
                lobby_id,
                password: None,
                command: LobbyCommand::UpdatePlayerId { player_id },
            },
            &connection,
        )
        .await;

        if let Err(e) = self.connection_repo.add_connection(connection).await {
            log::error!("Failed to add connection: {:?}", e);
        }
    }

    pub async fn handle_command(&self, command: &LobbyCommandWrapper) {
        match command.command {
            LobbyCommand::Join {
                player_id,
                lobby_id,
                ..
            } => {
                if let Err(e) = self
                    .lobby_repo
                    .add_player_to_lobby(lobby_id, player_id)
                    .await
                {
                    log::error!("Failed to add player to lobby: {:?}", e);
                } else {
                    log::info!("Player {} joined lobby {}", player_id, lobby_id);
                }
            }
            _ => {}
        }
    }

    pub async fn broadcast_to_lobby(&self, lobby_id: Uuid, command: &LobbyCommandWrapper) {
        match self.lobby_repo.get_players_in_lobby(lobby_id).await {
            Ok(player_ids) => {
                for player_id in player_ids {
                    if let Ok(Some(connection)) =
                        self.connection_repo.get_connection(player_id).await
                    {
                        self.send_command(command, &connection).await;
                    }
                }
            }
            Err(e) => log::error!("Failed to get players in lobby: {:?}", e),
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

        if let Err(e) = connection
            .sender
            .send(Message::Text(serialized_message))
            .await
        {
            log::error!(
                "Failed to send message to player {}: {:?}",
                connection.player_id,
                e
            );
        }
    }

    pub async fn remove_connection(&self, lobby_id: Uuid, player_id: Uuid) {
        if let Err(e) = self.connection_repo.remove_connection(player_id).await {
            log::error!("Failed to remove connection: {:?}", e);
        }

        if let Err(e) = self
            .lobby_repo
            .remove_player_from_lobby(lobby_id, player_id)
            .await
        {
            log::error!("Failed to remove player from lobby: {:?}", e);
        }
    }
}
