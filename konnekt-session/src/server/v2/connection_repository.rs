use super::Connection;
use crate::model::{ClientId, NetworkError};
use async_trait::async_trait;

#[async_trait]
pub trait ConnectionRepository: Send + Sync {
    async fn add_connection(&self, connection: Connection) -> Result<(), NetworkError>;
    async fn remove_connection(&self, id: ClientId) -> Result<(), NetworkError>;
    async fn get_connection(&self, id: ClientId) -> Result<Option<Connection>, NetworkError>;
    async fn get_all_connections(&self) -> Result<Vec<Connection>, NetworkError>;
}
