use super::super::connection::{ConnectionManager, PeerId};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct WsPeerId(pub String);
impl PeerId for WsPeerId {}

#[derive(Default, Clone)]
pub struct WebSocketConnectionManager {
    peer: Arc<RwLock<Option<WsPeerId>>>,
}

impl WebSocketConnectionManager {
    pub fn new() -> Self {
        Self {
            peer: Arc::new(RwLock::new(None)),
        }
    }
}

impl ConnectionManager for WebSocketConnectionManager {
    type Peer = WsPeerId;

    fn add_peer(&self, peer: Self::Peer) {
        *self.peer.write().unwrap() = Some(peer);
    }

    fn remove_peer(&self, peer: &Self::Peer) {
        if self.is_peer_connected(&peer) {
            *self.peer.write().unwrap() = None;
        }
    }

    fn get_connected_peers(&self) -> Vec<Self::Peer> {
        if let Some(peer) = self.peer.read().unwrap().as_ref() {
            vec![peer.clone()]
        } else {
            vec![]
        }
    }

    fn is_peer_connected(&self, peer: &Self::Peer) -> bool {
        if let Some(connected_peer) = self.peer.read().unwrap().as_ref() {
            connected_peer == peer
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_peer() {
        let manager = WebSocketConnectionManager::new();
        let peer = WsPeerId("ws://test".to_string());
        manager.add_peer(peer.clone());
        assert_eq!(manager.get_connected_peers(), vec![peer]);
    }

    #[test]
    fn test_remove_peer() {
        let manager = WebSocketConnectionManager::new();
        let peer = WsPeerId("ws://test".to_string());
        manager.add_peer(peer.clone());
        manager.remove_peer(&peer);
        assert_eq!(manager.get_connected_peers(), vec![]);
    }

    #[test]
    fn test_is_peer_connected() {
        let manager = WebSocketConnectionManager::new();
        let peer = WsPeerId("ws://test".to_string());
        manager.add_peer(peer.clone());
        assert!(manager.is_peer_connected(&peer));
    }
}
