use konnekt_session_p2p::application::ConnectionEvent;
use konnekt_session_p2p::domain::PeerId;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Mock connection that simulates P2P networking in-memory
#[derive(Clone)]
pub struct MockConnection {
    /// Our peer ID
    local_id: PeerId,

    /// Shared network bus (all peers write/read from here)
    network: Arc<Mutex<MockNetwork>>,

    /// Our local message queue
    inbox: Arc<Mutex<VecDeque<(PeerId, Vec<u8>)>>>,
}

/// Shared network bus (simulates WebRTC signalling + data channels)
pub struct MockNetwork {
    /// All registered peers
    pub peers: HashMap<PeerId, Arc<Mutex<VecDeque<(PeerId, Vec<u8>)>>>>,

    /// Connection events (peer connected/disconnected)
    pub events: VecDeque<(PeerId, ConnectionEvent)>,
}

impl MockConnection {
    /// Create a new mock connection
    pub fn new(network: Arc<Mutex<MockNetwork>>) -> Self {
        let local_id = PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()));
        let inbox = Arc::new(Mutex::new(VecDeque::new()));

        println!("🔌 MockConnection: New peer {} created", local_id);

        // Register with network
        network
            .lock()
            .unwrap()
            .peers
            .insert(local_id, inbox.clone());

        // Notify all existing peers
        let existing_peers: Vec<PeerId> = network
            .lock()
            .unwrap()
            .peers
            .keys()
            .filter(|&&id| id != local_id)
            .copied()
            .collect();

        println!("   ↳ Found {} existing peers", existing_peers.len());

        for peer_id in existing_peers {
            println!("   ↳ Notifying {} about {}", local_id, peer_id);
            println!("   ↳ Notifying {} about {}", peer_id, local_id);

            network
                .lock()
                .unwrap()
                .events
                .push_back((local_id, ConnectionEvent::PeerConnected(peer_id)));

            network
                .lock()
                .unwrap()
                .events
                .push_back((peer_id, ConnectionEvent::PeerConnected(local_id)));
        }

        Self {
            local_id,
            network,
            inbox,
        }
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> Option<PeerId> {
        Some(self.local_id)
    }

    /// Get connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        let peers: Vec<_> = self
            .network
            .lock()
            .unwrap()
            .peers
            .keys()
            .filter(|&&id| id != self.local_id)
            .copied()
            .collect();

        println!(
            "🔍 Peer {} sees {} connected peers",
            self.local_id,
            peers.len()
        );
        peers
    }

    /// Send to specific peer (✅ synchronous delivery)
    pub fn send_to(&mut self, peer: PeerId, data: Vec<u8>) -> Result<(), String> {
        tracing::trace!(
            "📤 Peer {} → Peer {} ({} bytes)",
            self.local_id,
            peer,
            data.len()
        );

        let network = self.network.lock().unwrap();

        if let Some(peer_inbox) = network.peers.get(&peer) {
            peer_inbox.lock().unwrap().push_back((self.local_id, data));
            Ok(())
        } else {
            Err(format!("Peer {} not found", peer))
        }
    }

    /// Broadcast to all peers (✅ synchronous delivery to all)
    pub fn broadcast(&mut self, data: Vec<u8>) -> Result<(), String> {
        let peers = self.connected_peers();
        tracing::trace!(
            "📢 Peer {} broadcasting to {} peers ({} bytes)",
            self.local_id,
            peers.len(),
            data.len()
        );

        // ✅ Deliver to all peers in one atomic operation
        for peer in peers {
            self.send_to(peer, data.clone())?;
        }
        Ok(())
    }

    /// Poll for events
    pub fn poll_events(&mut self) -> Vec<ConnectionEvent> {
        let mut events = Vec::new();

        // Get connection events for this peer
        let mut network = self.network.lock().unwrap();
        let mut peer_events = Vec::new();

        // Extract events for this peer
        let mut remaining = VecDeque::new();
        for (target, event) in network.events.drain(..) {
            if target == self.local_id {
                peer_events.push(event);
            } else {
                remaining.push_back((target, event));
            }
        }
        network.events = remaining;
        drop(network);

        events.extend(peer_events);

        // Get messages from inbox
        let mut inbox = self.inbox.lock().unwrap();
        let message_count = inbox.len();

        while let Some((from, data)) = inbox.pop_front() {
            println!(
                "📥 Peer {} ← Peer {} ({} bytes)",
                self.local_id,
                from,
                data.len()
            );
            events.push(ConnectionEvent::MessageReceived { from, data });
        }

        if !events.is_empty() || message_count > 0 {
            println!(
                "📊 Peer {} polled: {} events ({} messages)",
                self.local_id,
                events.len(),
                message_count
            );
        }

        events
    }
}

/// Create a mock network (shared between all peers)
pub fn create_mock_network() -> Arc<Mutex<MockNetwork>> {
    println!("🌐 Creating mock network");
    Arc::new(Mutex::new(MockNetwork {
        peers: HashMap::new(),
        events: VecDeque::new(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_connection_basic() {
        let network = create_mock_network();

        let mut peer1 = MockConnection::new(network.clone());
        let mut peer2 = MockConnection::new(network.clone());

        // Check peer IDs
        assert_ne!(peer1.local_peer_id(), peer2.local_peer_id());

        // Check connected peers
        assert_eq!(peer1.connected_peers().len(), 1);
        assert_eq!(peer2.connected_peers().len(), 1);

        // Send message
        let msg = b"Hello".to_vec();
        peer1
            .send_to(peer2.local_peer_id().unwrap(), msg.clone())
            .unwrap();

        // Receive message
        let events = peer2.poll_events();
        assert_eq!(events.len(), 2); // PeerConnected + MessageReceived

        match &events[1] {
            ConnectionEvent::MessageReceived { from, data } => {
                assert_eq!(*from, peer1.local_peer_id().unwrap());
                assert_eq!(*data, msg);
            }
            _ => panic!("Expected MessageReceived"),
        }
    }

    #[test]
    fn test_broadcast() {
        let network = create_mock_network();

        let mut peer1 = MockConnection::new(network.clone());
        let mut peer2 = MockConnection::new(network.clone());
        let mut peer3 = MockConnection::new(network.clone());

        // Broadcast from peer1
        let msg = b"Broadcast".to_vec();
        peer1.broadcast(msg.clone()).unwrap();

        // Both peer2 and peer3 should receive
        let events2 = peer2.poll_events();
        let events3 = peer3.poll_events();

        // Each peer gets 2 PeerConnected + 1 MessageReceived
        assert!(events2.len() >= 1);
        assert!(events3.len() >= 1);
    }
}
