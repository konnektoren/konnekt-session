use crate::application::ConnectionEvent;
use crate::domain::PeerId;
use crate::infrastructure::error::{P2PError, Result};
use crate::infrastructure::message::{MessageKind, P2PMessage};
use std::collections::{HashMap, VecDeque};

/// Events emitted by transport (for SessionLoop to handle)
#[derive(Debug, Clone)]
pub enum TransportEvent {
    /// Peer connected (for host to send snapshot)
    PeerConnected(PeerId),

    /// Received snapshot request (host should respond)
    SnapshotRequested { from: PeerId },

    /// Received snapshot response (guest applies)
    SnapshotReceived {
        snapshot: serde_json::Value,
        as_of_sequence: u64,
    },
}

/// Trait for network connection (allows mocking in tests)
pub trait NetworkConnection {
    fn local_peer_id(&self) -> Option<PeerId>;
    fn connected_peers(&self) -> Vec<PeerId>;
    fn send_to(&mut self, peer: PeerId, data: Vec<u8>) -> Result<()>;
    fn broadcast(&mut self, data: Vec<u8>) -> Result<()>;
    fn poll_events(&mut self) -> Vec<ConnectionEvent>;
}

/// Implement NetworkConnection for MatchboxConnection
impl NetworkConnection for crate::infrastructure::connection::MatchboxConnection {
    fn local_peer_id(&self) -> Option<PeerId> {
        self.local_peer_id()
    }

    fn connected_peers(&self) -> Vec<PeerId> {
        self.connected_peers()
    }

    fn send_to(&mut self, peer: PeerId, data: Vec<u8>) -> Result<()> {
        self.send_to(peer, data)
    }

    fn broadcast(&mut self, data: Vec<u8>) -> Result<()> {
        self.broadcast(data)
    }

    fn poll_events(&mut self) -> Vec<ConnectionEvent> {
        self.poll_events()
    }
}

/// Reliable P2P transport (domain-agnostic, generic over connection)
pub struct P2PTransport<C: NetworkConnection> {
    /// Network connection (can be real or mock)
    connection: C,

    /// Next sequence number to assign (host only)
    next_sequence: u64,

    /// Highest sequence received (all peers)
    highest_received: u64,

    /// Out-of-order messages waiting for gaps
    pending_messages: HashMap<u64, (PeerId, P2PMessage)>,

    /// Delivered messages (for resend requests)
    message_cache: VecDeque<P2PMessage>,

    /// Max cache size
    cache_size: usize,

    /// Are we the host?
    is_host: bool,

    /// Learned host peer (guest only). Used to route commands deterministically.
    host_peer: Option<PeerId>,

    /// Transport events (for SessionLoop)
    pending_events: Vec<TransportEvent>,

    /// V2 drives resend requests from its synchronization protocol.
    managed_recovery: bool,
}

impl<C: NetworkConnection> P2PTransport<C> {
    /// Create a new transport as host
    pub fn new_host(connection: C, cache_size: usize) -> Self {
        Self {
            connection,
            next_sequence: 1, // Start at 1 (0 reserved for control)
            highest_received: 0,
            pending_messages: HashMap::new(),
            message_cache: VecDeque::with_capacity(cache_size),
            cache_size,
            is_host: true,
            host_peer: None,
            pending_events: Vec::new(),
            managed_recovery: false,
        }
    }

    /// Create a new transport as guest
    pub fn new_guest(connection: C, cache_size: usize) -> Self {
        Self {
            connection,
            next_sequence: 0, // Guests don't assign sequences
            highest_received: 0,
            pending_messages: HashMap::new(),
            message_cache: VecDeque::new(),
            cache_size,
            is_host: false,
            host_peer: None,
            pending_events: Vec::new(),
            managed_recovery: false,
        }
    }

    /// Send an application message (HOST ONLY - broadcasts to ALL peers)
    pub fn send(&mut self, payload: serde_json::Value) -> Result<u64> {
        if !self.is_host {
            return Err(P2PError::SendFailed(
                "Only host can broadcast messages".to_string(),
            ));
        }

        let sequence = self.next_sequence;
        self.next_sequence += 1;

        let mut msg = P2PMessage::application(payload);
        msg.sequence = sequence;

        // Serialize and broadcast
        let data = serde_json::to_vec(&msg).map_err(P2PError::Serialization)?;

        // ✅ FIX: Broadcast to ALL connected peers (not including self)
        self.connection.broadcast(data)?;

        // Cache for resend
        self.message_cache.push_back(msg);
        if self.message_cache.len() > self.cache_size {
            self.message_cache.pop_front();
        }

        tracing::debug!("📡 HOST: Broadcast message with sequence {}", sequence);

        Ok(sequence)
    }

    /// Send directly to a peer (for guest → host)
    pub fn send_to_host(&mut self, payload: serde_json::Value) -> Result<()> {
        let msg = P2PMessage::application(payload);

        let data = serde_json::to_vec(&msg).map_err(P2PError::Serialization)?;

        let peers = self.connection.connected_peers();
        if peers.is_empty() {
            return Err(P2PError::SendFailed("No host connected".to_string()));
        }

        // Prefer the learned host peer; fallback for startup before snapshot response.
        let target = if let Some(host_peer) = self.host_peer {
            if peers.contains(&host_peer) {
                host_peer
            } else {
                let fallback = peers[0];
                self.host_peer = Some(fallback);
                fallback
            }
        } else {
            let fallback = peers[0];
            self.host_peer = Some(fallback);
            fallback
        };

        self.connection.send_to(target, data)?;

        Ok(())
    }

    /// Send a snapshot to a specific peer (host only)
    pub fn send_snapshot(&mut self, peer: PeerId, snapshot: serde_json::Value) -> Result<()> {
        if !self.is_host {
            return Err(P2PError::SendFailed(
                "Only host can send snapshots".to_string(),
            ));
        }

        let msg = P2PMessage::snapshot_response(snapshot, self.next_sequence - 1);
        let data = serde_json::to_vec(&msg).map_err(P2PError::Serialization)?;

        self.connection.send_to(peer, data)?;
        tracing::info!(
            "📤 Sent snapshot to peer {} (seq: {})",
            peer,
            self.next_sequence - 1
        );

        Ok(())
    }

    /// Request snapshot from host (guest only)
    pub fn request_snapshot(&mut self) -> Result<()> {
        if self.is_host {
            return Err(P2PError::SendFailed(
                "Host doesn't request snapshots".to_string(),
            ));
        }

        let msg = P2PMessage::snapshot_request();
        let data = serde_json::to_vec(&msg).map_err(P2PError::Serialization)?;

        self.connection.broadcast(data)?;
        tracing::info!("📤 Requested snapshot from host");

        Ok(())
    }

    /// Poll for application messages (handles ordering + gap detection)
    pub fn poll(&mut self) -> Vec<serde_json::Value> {
        self.poll_with_sender()
            .into_iter()
            .map(|(_, payload)| payload)
            .collect()
    }

    /// Poll application messages with the peer that delivered them.
    ///
    /// For a resend response this is the responding peer; it does not prove
    /// who originally authored a message or authenticate its contents.
    pub fn poll_with_sender(&mut self) -> Vec<(PeerId, serde_json::Value)> {
        let mut delivered = Vec::new();

        // Get raw network events
        for event in self.connection.poll_events() {
            match event {
                ConnectionEvent::PeerConnected(peer_id) => {
                    tracing::info!("🟢 Peer connected: {}", peer_id);
                    self.pending_events
                        .push(TransportEvent::PeerConnected(peer_id));
                }
                ConnectionEvent::MessageReceived { from, data } => {
                    if let Ok(msg) = serde_json::from_slice::<P2PMessage>(&data) {
                        match msg.kind {
                            MessageKind::Application { payload } => {
                                self.handle_application_message(
                                    from,
                                    msg.sequence,
                                    payload.clone(),
                                    &mut delivered,
                                );
                            }
                            MessageKind::SnapshotRequest => {
                                tracing::info!("📥 Received snapshot request from {}", from);
                                self.pending_events
                                    .push(TransportEvent::SnapshotRequested { from });
                            }
                            MessageKind::SnapshotResponse {
                                snapshot,
                                as_of_sequence,
                            } => {
                                if !self.is_host {
                                    // Snapshot responses are authoritative from host.
                                    self.host_peer = Some(from);
                                    // A restarted host may begin its sequence at zero again.
                                    self.highest_received = as_of_sequence;
                                    self.pending_messages.clear();
                                }
                                tracing::info!("📥 Received snapshot (seq: {})", as_of_sequence);
                                self.pending_events.push(TransportEvent::SnapshotReceived {
                                    snapshot,
                                    as_of_sequence,
                                });
                            }
                            MessageKind::ResendRequest { from: seq_from, to } => {
                                self.handle_resend_request(seq_from, to, from);
                            }
                            MessageKind::ResendResponse { messages } => {
                                self.handle_resend_response(from, messages, &mut delivered);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        delivered
    }

    /// Drain transport events (for SessionLoop)
    pub fn drain_events(&mut self) -> Vec<TransportEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Handle application message with ordering
    fn handle_application_message(
        &mut self,
        from: PeerId,
        sequence: u64,
        payload: serde_json::Value,
        delivered: &mut Vec<(PeerId, serde_json::Value)>,
    ) {
        if sequence == 0 {
            // Unsequenced message (from guest) - deliver immediately
            delivered.push((from, payload));
            return;
        }

        if sequence == self.highest_received + 1 {
            // In order - deliver immediately
            delivered.push((from, payload));
            self.highest_received = sequence;

            // Check if we can deliver pending messages
            while let Some((from, pending)) =
                self.pending_messages.remove(&(self.highest_received + 1))
            {
                if let MessageKind::Application { payload } = pending.kind {
                    delivered.push((from, payload));
                    self.highest_received = pending.sequence;
                }
            }
        } else if sequence > self.highest_received + 1 {
            // Out of order - buffer it
            let msg = P2PMessage {
                sequence,
                kind: MessageKind::Application { payload },
            };
            self.pending_messages.insert(sequence, (from, msg));

            // Request missing range
            if !self.managed_recovery {
                self.request_resend(self.highest_received + 1, sequence - 1);
            }
        }
        // else: duplicate/old message, ignore
    }

    /// Handle resend request (host only)
    fn handle_resend_request(&mut self, from: u64, to: u64, peer: PeerId) {
        if !self.is_host {
            return;
        }

        let messages: Vec<P2PMessage> = self
            .message_cache
            .iter()
            .filter(|msg| msg.sequence >= from && msg.sequence <= to)
            .cloned()
            .collect();

        if !messages.is_empty() {
            let response = P2PMessage {
                sequence: 0,
                kind: MessageKind::ResendResponse { messages },
            };

            if let Ok(data) = serde_json::to_vec(&response) {
                let _ = self.connection.send_to(peer, data);
            }
        }
    }

    /// Handle resend response (guest only)
    fn handle_resend_response(
        &mut self,
        from: PeerId,
        messages: Vec<P2PMessage>,
        delivered: &mut Vec<(PeerId, serde_json::Value)>,
    ) {
        for msg in messages {
            // Extract sequence first, then handle payload
            let sequence = msg.sequence;
            if let MessageKind::Application { payload } = msg.kind {
                self.handle_application_message(from, sequence, payload, delivered);
            }
        }
    }

    /// Request resend of missing messages
    fn request_resend(&mut self, from: u64, to: u64) {
        let request = P2PMessage::resend_request(from, to);

        if let Ok(data) = serde_json::to_vec(&request) {
            let _ = self.connection.broadcast(data);
        }
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> Option<PeerId> {
        self.connection.local_peer_id()
    }

    /// Get connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.connection.connected_peers()
    }

    pub(crate) fn has_sequence_gap(&self) -> bool {
        !self.pending_messages.is_empty()
    }

    pub(crate) fn use_managed_recovery(&mut self) {
        self.managed_recovery = true;
    }

    pub(crate) fn missing_range(&self) -> Option<(u64, u64)> {
        let first_buffered = self.pending_messages.keys().min().copied()?;
        let from = self.highest_received.checked_add(1)?;
        (from < first_buffered).then_some((from, first_buffered - 1))
    }

    pub(crate) fn request_missing_messages(&mut self, from: u64, to: u64) {
        self.request_resend(from, to);
    }
}

// Type alias for production use (with MatchboxConnection)
pub type MatchboxP2PTransport = P2PTransport<crate::infrastructure::connection::MatchboxConnection>;

#[cfg(test)]
mod tests {
    use super::*;

    struct EmptyConnection;

    impl NetworkConnection for EmptyConnection {
        fn local_peer_id(&self) -> Option<PeerId> {
            None
        }
        fn connected_peers(&self) -> Vec<PeerId> {
            Vec::new()
        }
        fn send_to(&mut self, _: PeerId, _: Vec<u8>) -> Result<()> {
            Ok(())
        }
        fn broadcast(&mut self, _: Vec<u8>) -> Result<()> {
            Ok(())
        }
        fn poll_events(&mut self) -> Vec<ConnectionEvent> {
            Vec::new()
        }
    }

    #[test]
    fn preserves_sender_when_buffered_messages_are_delivered() {
        let mut transport = P2PTransport::new_guest(EmptyConnection, 10);
        let first = PeerId::new(matchbox_socket::PeerId(uuid::Uuid::new_v4()));
        let second = PeerId::new(matchbox_socket::PeerId(uuid::Uuid::new_v4()));
        let mut delivered = Vec::new();
        transport.handle_application_message(second, 2, serde_json::json!(2), &mut delivered);
        assert!(delivered.is_empty());
        transport.handle_application_message(first, 1, serde_json::json!(1), &mut delivered);
        assert_eq!(
            delivered,
            vec![
                (first, serde_json::json!(1)),
                (second, serde_json::json!(2))
            ]
        );
    }

    #[test]
    fn resend_response_records_the_sending_peer() {
        let mut transport = P2PTransport::new_guest(EmptyConnection, 10);
        let peer = PeerId::new(matchbox_socket::PeerId(uuid::Uuid::new_v4()));
        let mut message = P2PMessage::application(serde_json::json!("resent"));
        message.sequence = 1;
        let mut delivered = Vec::new();
        transport.handle_resend_response(peer, vec![message], &mut delivered);
        assert_eq!(delivered, vec![(peer, serde_json::json!("resent"))]);
    }

    #[test]
    fn test_transport_event_types() {
        // Test that we can create transport events
        let peer = PeerId::new(matchbox_socket::PeerId(uuid::Uuid::new_v4()));
        let event = TransportEvent::PeerConnected(peer);

        match event {
            TransportEvent::PeerConnected(id) => assert_eq!(id, peer),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_snapshot_request_event() {
        let peer = PeerId::new(matchbox_socket::PeerId(uuid::Uuid::new_v4()));
        let event = TransportEvent::SnapshotRequested { from: peer };

        match event {
            TransportEvent::SnapshotRequested { from } => {
                assert_eq!(from, peer);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_snapshot_received_event() {
        let snapshot = serde_json::json!({"test": "data"});
        let event = TransportEvent::SnapshotReceived {
            snapshot: snapshot.clone(),
            as_of_sequence: 42,
        };

        match event {
            TransportEvent::SnapshotReceived { as_of_sequence, .. } => {
                assert_eq!(as_of_sequence, 42);
            }
            _ => panic!("Wrong event type"),
        }
    }
}
