use crate::model::{ClientId, LobbyId};
use axum::extract::ws::Message;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct Connection {
    pub client_id: ClientId,
    pub lobby_id: LobbyId,
    pub sender: Sender<Message>,
}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        self.client_id == other.client_id && self.lobby_id == other.lobby_id
    }
}

impl Connection {
    pub fn new(client_id: ClientId, lobby_id: LobbyId, sender: Sender<Message>) -> Self {
        Connection {
            client_id,
            lobby_id,
            sender,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LobbyId;
    use tokio::sync::mpsc::channel;

    #[test]
    fn test_partial_eq() {
        let client_id = ClientId::new_v4();
        let lobby_id = LobbyId::new_v4();
        let sender = channel(1).0;
        let connection = Connection::new(client_id, lobby_id, sender.clone());
        let connection2 = Connection::new(client_id, lobby_id, sender);
        assert_eq!(connection, connection2);
    }
}
