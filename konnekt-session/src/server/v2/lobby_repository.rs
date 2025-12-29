use crate::model::{ClientId, LobbyId, NetworkError};
use async_trait::async_trait;

#[async_trait]
pub trait LobbyRepository: Send + Sync {
    async fn add_client_to_lobby(
        &self,
        lobby_id: LobbyId,
        client_id: ClientId,
    ) -> Result<(), NetworkError>;
    async fn remove_client_from_lobby(
        &self,
        lobby_id: LobbyId,
        client_id: ClientId,
    ) -> Result<(), NetworkError>;
    async fn get_clients_in_lobby(&self, lobby_id: LobbyId) -> Result<Vec<ClientId>, NetworkError>;

    async fn get_lobby_with_client(
        &self,
        client_id: ClientId,
    ) -> Result<Option<LobbyId>, NetworkError>;
}
