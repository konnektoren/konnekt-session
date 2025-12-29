use crate::domain::PeerId;
use uuid::Uuid;

/// Events emitted by the P2P connection
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// A new peer has connected
    PeerConnected(PeerId),

    /// A peer has disconnected (grace period started)
    PeerDisconnected(PeerId),

    /// A peer's grace period has expired
    PeerTimedOut {
        peer_id: PeerId,
        participant_id: Option<Uuid>,
        was_host: bool,
    },

    /// Received a message from a peer
    MessageReceived { from: PeerId, data: Vec<u8> },
}
