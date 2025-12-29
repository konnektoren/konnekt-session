use crate::{P2PError, Result};
use matchbox_socket::{PeerId as MatchboxPeerId, WebRtcSocket};
use serde::{Deserialize, Serialize};
use std::fmt;
use tokio::sync::mpsc;

/// Unique identifier for a peer in the P2P network
/// Re-exports matchbox's PeerId which is already a Uuid wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub MatchboxPeerId);

impl PeerId {
    pub fn new(id: MatchboxPeerId) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
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

/// Events emitted by the P2P connection
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// A new peer has connected
    PeerConnected(PeerId),
    /// A peer has disconnected
    PeerDisconnected(PeerId),
    /// Received a message from a peer
    MessageReceived { from: PeerId, data: Vec<u8> },
}

/// Manages WebRTC connection via Matchbox signalling
pub struct Connection {
    socket: WebRtcSocket,
    local_peer_id: PeerId,
    event_tx: mpsc::UnboundedSender<ConnectionEvent>,
    event_rx: mpsc::UnboundedReceiver<ConnectionEvent>,
}

impl Connection {
    /// Connect to Matchbox signalling server
    pub async fn connect(signalling_url: &str) -> Result<Self> {
        tracing::info!("Connecting to signalling server: {}", signalling_url);

        let (mut socket, _loop_fut) = WebRtcSocket::new_reliable(signalling_url);

        // Get our local peer ID
        let local_peer_id = socket
            .id()
            .ok_or_else(|| P2PError::ConnectionFailed("No peer ID assigned".to_string()))?;

        tracing::info!("Connected with peer ID: {}", local_peer_id);

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Ok(Connection {
            socket,
            local_peer_id: PeerId::new(local_peer_id),
            event_tx,
            event_rx,
        })
    }

    /// Get our local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Get list of currently connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.socket.connected_peers().map(PeerId::new).collect()
    }

    /// Send data to a specific peer
    pub fn send_to(&mut self, peer: PeerId, data: Vec<u8>) -> Result<()> {
        self.socket.send(data.clone().into_boxed_slice(), peer.0);

        tracing::debug!("Sent {} bytes to peer {}", data.len(), peer);
        Ok(())
    }

    /// Broadcast data to all connected peers
    pub fn broadcast(&mut self, data: Vec<u8>) -> Result<()> {
        let peer_count = self.connected_peers().len();

        for peer in self.connected_peers() {
            self.send_to(peer, data.clone())?;
        }

        tracing::debug!("Broadcast {} bytes to {} peers", data.len(), peer_count);
        Ok(())
    }

    /// Poll for events (call this regularly in your event loop)
    pub fn poll_events(&mut self) -> Vec<ConnectionEvent> {
        let mut events = Vec::new();

        // Check for new peers
        for (peer_id, state) in self.socket.update_peers() {
            let peer = PeerId::new(peer_id);
            match state {
                matchbox_socket::PeerState::Connected => {
                    tracing::info!("Peer connected: {}", peer);
                    events.push(ConnectionEvent::PeerConnected(peer));
                }
                matchbox_socket::PeerState::Disconnected => {
                    tracing::info!("Peer disconnected: {}", peer);
                    events.push(ConnectionEvent::PeerDisconnected(peer));
                }
            }
        }

        // Check for messages
        for (peer_id, packet) in self.socket.receive() {
            let peer = PeerId::new(peer_id);
            tracing::debug!("Received {} bytes from peer {}", packet.len(), peer);

            events.push(ConnectionEvent::MessageReceived {
                from: peer,
                data: packet.to_vec(),
            });
        }

        events
    }

    /// Subscribe to connection events
    pub fn subscribe(&self) -> mpsc::UnboundedReceiver<ConnectionEvent> {
        let (_tx, rx) = mpsc::unbounded_channel();
        rx
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
    fn test_connection_event_debug() {
        let uuid = Uuid::new_v4();
        let peer = PeerId(MatchboxPeerId(uuid));

        let event1 = ConnectionEvent::PeerConnected(peer);
        let event2 = ConnectionEvent::PeerDisconnected(peer);
        let event3 = ConnectionEvent::MessageReceived {
            from: peer,
            data: vec![1, 2, 3],
        };

        // Just verify they can be debug-printed
        format!("{:?}", event1);
        format!("{:?}", event2);
        format!("{:?}", event3);
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
