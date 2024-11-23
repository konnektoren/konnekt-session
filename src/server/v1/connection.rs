use crate::model::Role;
use axum::extract::ws::Message;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Connection {
    pub player_id: Uuid,
    pub sender: Sender<Message>,
    pub lobby_id: Uuid,
    pub role: Role,
}
