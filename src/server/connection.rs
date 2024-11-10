use crate::model::Role;
use axum::extract::ws::Message;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Connection {
    pub player_id: Uuid,
    pub sender: UnboundedSender<Message>,
    pub lobby_id: Uuid,
    pub role: Role,
}
