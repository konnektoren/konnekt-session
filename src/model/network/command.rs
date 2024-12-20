use crate::model::LobbyId;

use super::ClientId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NetworkCommand<T> {
    Connect {
        client_id: ClientId,
        lobby_id: LobbyId,
    },
    Disconnect {
        client_id: ClientId,
        lobby_id: LobbyId,
    },
    Ping {
        id: Uuid,
        client_id: ClientId,
    },
    Pong {
        id: Uuid,
        client_id: ClientId,
    },
    Message {
        client_id: ClientId,
        data: T,
    },
}

impl<T> NetworkCommand<T> {
    pub fn get_type(&self) -> &str {
        match self {
            NetworkCommand::Connect { .. } => "Connect",
            NetworkCommand::Disconnect { .. } => "Disconnect",
            NetworkCommand::Message { .. } => "Message",
            NetworkCommand::Ping { .. } => "Ping",
            NetworkCommand::Pong { .. } => "Pong",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Data {
            pub field: String,
        }

        let data = Data {
            field: "test".to_string(),
        };

        let command = NetworkCommand::Message {
            client_id: ClientId::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8").unwrap(),
            data,
        };
        let serialized = serde_json::to_string(&command).unwrap();
        assert_eq!(
            serialized,
            r#"{"Message":{"client_id":"a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8","data":{"field":"test"}}}"#
        );
    }
}
