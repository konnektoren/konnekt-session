use super::super::connection::{ConnectionManager, PeerId};
use matchbox_socket::PeerId as MatchboxPeerId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

impl PeerId for MatchboxPeerId {}

pub struct MatchboxConnectionManager {
    peers: Arc<RwLock<HashMap<MatchboxPeerId, bool>>>,
}

impl MatchboxConnectionManager {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl ConnectionManager for MatchboxConnectionManager {
    type Peer = MatchboxPeerId;

    fn add_peer(&self, peer: Self::Peer) {
        self.peers.write().unwrap().insert(peer, false);
    }

    fn remove_peer(&self, peer: &Self::Peer) {
        self.peers.write().unwrap().remove(peer);
    }

    fn get_connected_peers(&self) -> Vec<Self::Peer> {
        self.peers
            .read()
            .unwrap()
            .iter()
            .filter(|(_, connected)| **connected)
            .map(|(peer, _)| *peer)
            .collect()
    }

    fn is_peer_connected(&self, peer: &Self::Peer) -> bool {
        *self.peers.read().unwrap().get(peer).unwrap_or(&false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matchbox_socket::PeerId as MatchboxPeerId;
    use uuid::Uuid;

    #[test]
    fn test_matchbox_connection_manager() {
        let manager = MatchboxConnectionManager::new();
        let peer1 = MatchboxPeerId(Uuid::new_v4());
        let peer2 = MatchboxPeerId(Uuid::new_v4());

        assert_eq!(manager.get_connected_peers().len(), 0);

        manager.add_peer(peer1);
        manager.add_peer(peer2);

        assert_eq!(manager.get_connected_peers().len(), 0);

        manager.remove_peer(&peer1);

        assert_eq!(manager.get_connected_peers().len(), 0);

        manager.peers.write().unwrap().insert(peer2, true);

        assert_eq!(manager.get_connected_peers().len(), 1);
        assert!(manager.is_peer_connected(&peer2));
    }
}
