use serde::{Deserialize, Serialize};
use std::fmt;

// Re-export the underlying matchbox type
pub use matchbox_socket::PeerId as MatchboxPeerId;

/// Domain entity: Unique identifier for a peer in the P2P network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub MatchboxPeerId);

impl PeerId {
    pub fn new(id: MatchboxPeerId) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }

    pub fn inner(&self) -> MatchboxPeerId {
        self.0
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<MatchboxPeerId> for PeerId {
    fn from(id: MatchboxPeerId) -> Self {
        Self(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_peer_id_display() {
        let uuid = Uuid::new_v4();
        let peer_id = PeerId(MatchboxPeerId(uuid));
        let display = peer_id.to_string();
        assert!(!display.is_empty());
        assert_eq!(display, uuid.to_string());
    }

    #[test]
    fn test_peer_id_equality() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let id1 = PeerId(MatchboxPeerId(uuid1));
        let id2 = PeerId(MatchboxPeerId(uuid1));
        let id3 = PeerId(MatchboxPeerId(uuid2));

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_peer_id_serialization() {
        let uuid = Uuid::new_v4();
        let peer = PeerId(MatchboxPeerId(uuid));

        let json = serde_json::to_string(&peer).unwrap();
        let deserialized: PeerId = serde_json::from_str(&json).unwrap();

        assert_eq!(peer, deserialized);
    }
}
