use matchbox_socket::PeerId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Default, Clone)]
pub struct MatchboxPeerManager {
    peers: Arc<RwLock<HashMap<PeerId, bool>>>, // bool represents connection status
}

impl MatchboxPeerManager {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_peer(&self, peer_id: PeerId) {
        self.peers.write().unwrap().insert(peer_id, true);
    }

    pub fn remove_peer(&self, peer_id: &PeerId) {
        self.peers.write().unwrap().remove(peer_id);
    }

    pub fn get_connected_peers(&self) -> Vec<PeerId> {
        self.peers
            .read()
            .unwrap()
            .iter()
            .filter(|(_, &connected)| connected)
            .map(|(peer_id, _)| peer_id.clone())
            .collect()
    }
}
