use super::{error::RepositoryError, Connection};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait ConnectionRepository: Send + Sync {
    async fn add_connection(&self, connection: Connection) -> Result<(), RepositoryError>;
    async fn remove_connection(&self, player_id: Uuid) -> Result<(), RepositoryError>;
    async fn get_connection(&self, player_id: Uuid) -> Result<Option<Connection>, RepositoryError>;
    async fn get_all_connections(&self) -> Result<Vec<Connection>, RepositoryError>;
}
