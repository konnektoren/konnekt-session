use super::error::RepositoryError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait LobbyRepository: Send + Sync {
    async fn add_player_to_lobby(
        &self,
        lobby_id: Uuid,
        player_id: Uuid,
    ) -> Result<(), RepositoryError>;
    async fn remove_player_from_lobby(
        &self,
        lobby_id: Uuid,
        player_id: Uuid,
    ) -> Result<(), RepositoryError>;
    async fn get_players_in_lobby(&self, lobby_id: Uuid) -> Result<Vec<Uuid>, RepositoryError>;
    async fn get_all_lobbies(&self) -> Result<Vec<(Uuid, Vec<Uuid>)>, RepositoryError>;
}
