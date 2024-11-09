use crate::model::Role;
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

#[derive(Debug)]
pub struct Connection {
    pub sender: UnboundedSender<Message>,
    pub player_id: Uuid,
    pub lobby_id: Uuid,
    pub role: Role,
}
