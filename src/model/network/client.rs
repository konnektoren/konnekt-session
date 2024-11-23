use crate::model::LobbyId;
use uuid::Uuid;

pub type ClientId = Uuid;

#[derive(Debug, Clone)]
pub struct Client {
    pub id: ClientId,
    pub lobby_id: LobbyId,
    pub ping: u32,
}
