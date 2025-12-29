use crate::domain::PeerId;

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
