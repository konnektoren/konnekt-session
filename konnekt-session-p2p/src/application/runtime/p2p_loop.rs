use crate::application::ConnectionEvent;
use crate::application::runtime::{MessageQueue, QueueError};
use crate::application::sync_manager::{EventSyncManager, SyncMessage, SyncResponse};
use crate::domain::{DomainEvent as P2PDomainEvent, LobbyEvent, PeerId, PeerRegistry};
use crate::infrastructure::connection::MatchboxConnection;
use crate::infrastructure::error::Result;
use instant::Duration;
use uuid::Uuid;

/// P2P event loop - AUTOMATICALLY handles event synchronization
pub struct P2PLoop {
    /// WebRTC connection (Matchbox adapter)
    connection: MatchboxConnection,

    /// Peer registry (tracks connection state)
    peer_registry: PeerRegistry,

    /// Event synchronization manager (automatic)
    event_sync: EventSyncManager,

    /// Outbound message queue (from application → network)
    outbound: MessageQueue,

    /// Inbound event queue (network events for caller)
    inbound_events: Vec<ConnectionEvent>,

    /// Inbound lobby events queue (parsed and validated events for application)
    inbound_lobby_events: Vec<LobbyEvent>,

    /// Max events to process per poll
    batch_size: usize,
}

impl P2PLoop {
    /// Create a new P2P loop as HOST
    pub fn new_host(
        connection: MatchboxConnection,
        lobby_id: Uuid,
        batch_size: usize,
        max_queue_size: usize,
    ) -> Self {
        tracing::info!("🎯 P2PLoop initialized as HOST for lobby {}", lobby_id);
        Self {
            connection,
            peer_registry: PeerRegistry::with_grace_period(Duration::from_secs(30)),
            event_sync: EventSyncManager::new_host(lobby_id),
            outbound: MessageQueue::new(max_queue_size),
            inbound_events: Vec::new(),
            inbound_lobby_events: Vec::new(),
            batch_size,
        }
    }

    /// Create a new P2P loop as GUEST
    pub fn new_guest(
        connection: MatchboxConnection,
        lobby_id: Uuid,
        batch_size: usize,
        max_queue_size: usize,
    ) -> Self {
        tracing::info!("🎯 P2PLoop initialized as GUEST for lobby {}", lobby_id);
        Self {
            connection,
            peer_registry: PeerRegistry::with_grace_period(Duration::from_secs(30)),
            event_sync: EventSyncManager::new_guest(lobby_id),
            outbound: MessageQueue::new(max_queue_size),
            inbound_events: Vec::new(),
            inbound_lobby_events: Vec::new(),
            batch_size,
        }
    }

    /// Submit a domain event to broadcast (HOST ONLY)
    ///
    /// This automatically:
    /// - Assigns sequence number
    /// - Wraps in LobbyEvent
    /// - Queues for broadcast
    /// Submit a domain event to broadcast (HOST ONLY)
    pub fn broadcast_event(&mut self, event: P2PDomainEvent) -> Result<u64> {
        let sync_msg = self
            .event_sync
            .create_event(event)
            .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))?;

        match sync_msg {
            SyncMessage::EventBroadcast { event } => {
                let sequence = event.sequence;

                // 🔧 FIX: Serialize before matching to avoid partial move
                let data = serde_json::to_vec(&SyncMessage::EventBroadcast { event })
                    .map_err(|e| crate::infrastructure::error::P2PError::Serialization(e))?;

                self.connection.broadcast(data)?;

                tracing::info!("📤 Broadcast event sequence {}", sequence);
                Ok(sequence)
            }
            _ => Err(crate::infrastructure::error::P2PError::SendFailed(
                "Unexpected sync message".to_string(),
            )),
        }
    }

    /// Process network events and send queued messages
    ///
    /// This AUTOMATICALLY handles:
    /// - Event ordering
    /// - Gap detection
    /// - State reconciliation
    ///
    /// Returns number of events processed
    pub fn poll(&mut self) -> usize {
        let mut processed = 0;

        // 1. Poll connection for network events
        let connection_events = self.connection.poll_events();

        for event in connection_events {
            processed += 1;

            match &event {
                ConnectionEvent::PeerConnected(peer_id) => {
                    self.peer_registry.add_peer(*peer_id);
                    tracing::debug!("✅ Added peer {} to registry", peer_id);
                }
                ConnectionEvent::MessageReceived { from, data } => {
                    self.peer_registry.update_last_seen(from);

                    // Try to parse as SyncMessage
                    if let Ok(sync_msg) = serde_json::from_slice::<SyncMessage>(data) {
                        tracing::debug!("📥 Received sync message from {}", from);

                        match self.event_sync.handle_message(*from, sync_msg) {
                            Ok(SyncResponse::ApplyEvents { events }) => {
                                tracing::info!("✅ Applying {} events from sync", events.len());
                                self.inbound_lobby_events.extend(events);
                            }
                            Ok(SyncResponse::SendMessage { to, message }) => {
                                // Auto-respond to sync requests
                                if let Ok(data) = serde_json::to_vec(&message) {
                                    if let Some(peer) = to {
                                        // 🔧 FIX: Use peer.inner() to convert to matchbox PeerId
                                        let _ = self.connection.send_to(PeerId(peer.inner()), data);
                                    } else {
                                        let _ = self.connection.broadcast(data);
                                    }
                                }
                            }
                            Ok(SyncResponse::ApplySnapshot {
                                snapshot: _,
                                events,
                            }) => {
                                tracing::info!("✅ Applying snapshot + {} events", events.len());
                                self.inbound_lobby_events.extend(events);
                            }
                            Ok(SyncResponse::NeedSnapshot {
                                for_peer,
                                since_sequence,
                            }) => {
                                tracing::info!(
                                    "📤 Peer {} needs snapshot from sequence {}",
                                    for_peer,
                                    since_sequence
                                );
                                // Application layer will need to provide snapshot
                                // For now, just log
                            }
                            Ok(SyncResponse::None) => {
                                // No action needed
                            }
                            Err(e) => {
                                tracing::warn!("❌ Failed to handle sync message: {:?}", e);
                            }
                        }
                    }
                }
                ConnectionEvent::PeerDisconnected(peer_id) => {
                    self.peer_registry.mark_peer_disconnected(peer_id);
                    tracing::debug!("🔴 Marked peer {} as disconnected", peer_id);
                }
                ConnectionEvent::PeerTimedOut { peer_id, .. } => {
                    self.peer_registry.remove_peer(peer_id);
                    tracing::debug!("⏰ Removed peer {} after timeout", peer_id);
                }
            }

            // Store event for caller
            self.inbound_events.push(event);
        }

        // 2. Check for grace period timeouts
        let timed_out_peers = self.peer_registry.check_grace_periods();
        for peer_id in timed_out_peers {
            if let Some(peer_state) = self.peer_registry.get_peer(&peer_id) {
                let participant_id = peer_state.participant_id;
                let was_host = peer_state.is_host;

                tracing::warn!("⏰ Peer {} timed out (was_host: {})", peer_id, was_host);

                self.inbound_events.push(ConnectionEvent::PeerTimedOut {
                    peer_id,
                    participant_id,
                    was_host,
                });

                processed += 1;
            }

            self.peer_registry.remove_peer(&peer_id);
        }

        // 3. Send outbound messages (up to batch_size)
        let mut sent = 0;
        while sent < self.batch_size {
            match self.outbound.pop() {
                Some(msg) => {
                    if let Ok(data) = serde_json::to_vec(&msg) {
                        if let Err(e) = self.connection.broadcast(data) {
                            tracing::error!("❌ Failed to broadcast message: {:?}", e);
                        } else {
                            tracing::debug!("📤 Broadcast LobbyEvent sequence {}", msg.sequence);
                        }
                    }
                    sent += 1;
                }
                None => break,
            }
        }

        processed + sent
    }

    /// Drain network events (for caller to process)
    pub fn drain_events(&mut self) -> Vec<ConnectionEvent> {
        std::mem::take(&mut self.inbound_events)
    }

    /// Drain parsed lobby events (AUTOMATIC - already ordered and validated)
    pub fn drain_lobby_events(&mut self) -> Vec<LobbyEvent> {
        std::mem::take(&mut self.inbound_lobby_events)
    }

    /// Get reference to peer registry
    pub fn peer_registry(&self) -> &PeerRegistry {
        &self.peer_registry
    }

    /// Get mutable reference to peer registry
    pub fn peer_registry_mut(&mut self) -> &mut PeerRegistry {
        &mut self.peer_registry
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> Option<PeerId> {
        self.connection.local_peer_id()
    }

    /// Get connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.connection.connected_peers()
    }

    /// Get pending message count
    pub fn pending_messages(&self) -> usize {
        self.outbound.len()
    }

    /// Get pending event count
    pub fn pending_events(&self) -> usize {
        self.inbound_events.len()
    }

    /// Get current sequence number (for debugging)
    pub fn current_sequence(&self) -> u64 {
        self.event_sync.current_sequence()
    }

    /// Promote to host (after delegation)
    pub fn promote_to_host(&mut self) {
        self.event_sync.promote_to_host();
        tracing::info!("👑 Promoted to HOST in P2P layer");
    }

    /// Send full sync to a specific peer (HOST ONLY)
    ///
    /// This should be called when a new peer connects
    pub fn send_full_sync_to_peer(
        &mut self,
        peer_id: PeerId,
        snapshot: crate::application::LobbySnapshot,
    ) -> Result<()> {
        let sync_msg = self
            .event_sync
            .create_full_sync_response(0, snapshot)
            .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))?;

        let data = serde_json::to_vec(&sync_msg)
            .map_err(|e| crate::infrastructure::error::P2PError::Serialization(e))?;

        self.connection.send_to(PeerId(peer_id.inner()), data)?;

        tracing::info!("📤 Sent full sync to peer {}", peer_id);
        Ok(())
    }

    /// Auto-send full sync to new peers (if we have a lobby)
    ///
    /// Call this in poll() when PeerConnected event occurs
    fn handle_peer_connected(&mut self, peer_id: PeerId) {
        // If we're host and have a lobby, send full sync
        // This is a hook for the application layer to provide snapshot
        // For now, just log
        tracing::debug!("📥 New peer connected: {}", peer_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These are unit tests for the loop structure.
    // Integration tests with real connections are in tests/

    #[test]
    fn test_message_queue_operations() {
        let mut queue = MessageQueue::new(10);

        let lobby_id = uuid::Uuid::new_v4();
        let event = LobbyEvent::new(
            1,
            lobby_id,
            crate::domain::DomainEvent::LobbyCreated {
                lobby_id,
                host_id: uuid::Uuid::new_v4(),
                name: "Test".to_string(),
            },
        );

        queue.push(event.clone()).unwrap();
        assert_eq!(queue.len(), 1);

        let popped = queue.pop().unwrap();
        assert_eq!(popped.sequence, 1);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_queue_overflow() {
        let mut queue = MessageQueue::new(2);
        let lobby_id = uuid::Uuid::new_v4();

        for i in 1..=2 {
            queue
                .push(LobbyEvent::new(
                    i,
                    lobby_id,
                    crate::domain::DomainEvent::GuestLeft {
                        participant_id: uuid::Uuid::new_v4(),
                    },
                ))
                .unwrap();
        }

        let result = queue.push(LobbyEvent::new(
            3,
            lobby_id,
            crate::domain::DomainEvent::GuestLeft {
                participant_id: uuid::Uuid::new_v4(),
            },
        ));

        assert!(result.is_err());
    }
}
