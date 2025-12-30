use crate::application::runtime::{MessageQueue, QueueError};
use crate::application::sync_manager::{EventSyncManager, SyncMessage, SyncResponse};
use crate::application::{ConnectionEvent, EventTranslator, LobbySnapshot};
use crate::domain::{DomainEvent as P2PDomainEvent, LobbyEvent, PeerId, PeerRegistry};
use crate::infrastructure::connection::MatchboxConnection;
use crate::infrastructure::error::Result;
use instant::Duration;
use konnekt_session_core::{DomainCommand, DomainEvent as CoreDomainEvent};
use std::collections::VecDeque;
use uuid::Uuid;

/// P2P event loop - AUTOMATICALLY handles event synchronization AND domain integration
pub struct P2PLoop {
    /// WebRTC connection (Matchbox adapter)
    connection: MatchboxConnection,

    /// Peer registry (tracks connection state)
    peer_registry: PeerRegistry,

    /// Event synchronization manager (automatic)
    event_sync: EventSyncManager,

    /// Event translator (P2P ↔ Core domain)
    translator: EventTranslator,

    /// Outbound message queue (from application → network)
    outbound: MessageQueue,

    /// Inbound event queue (network events for caller)
    inbound_events: Vec<ConnectionEvent>,

    /// Inbound lobby events queue (parsed and validated events for application)
    inbound_lobby_events: Vec<LobbyEvent>,

    /// Domain commands translated from incoming P2P events (for core to process)
    pending_domain_commands: VecDeque<DomainCommand>,

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
            translator: EventTranslator::new(lobby_id),
            outbound: MessageQueue::new(max_queue_size),
            inbound_events: Vec::new(),
            inbound_lobby_events: Vec::new(),
            pending_domain_commands: VecDeque::new(),
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
            translator: EventTranslator::new(lobby_id),
            outbound: MessageQueue::new(max_queue_size),
            inbound_events: Vec::new(),
            inbound_lobby_events: Vec::new(),
            pending_domain_commands: VecDeque::new(),
            batch_size,
        }
    }

    /// Request full sync from host (GUEST ONLY)
    pub fn request_full_sync(&mut self) -> Result<()> {
        let sync_msg = self
            .event_sync
            .request_full_sync()
            .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))?;

        // Serialize and broadcast
        let data = serde_json::to_vec(&sync_msg)
            .map_err(|e| crate::infrastructure::error::P2PError::Serialization(e))?;

        self.connection.broadcast(data)?;

        tracing::info!("📤 Sent full sync request to host");
        Ok(())
    }

    /// Apply snapshot to domain layer (converts snapshot to domain commands)
    fn apply_snapshot_to_domain(&mut self, snapshot: LobbySnapshot, events: Vec<LobbyEvent>) {
        tracing::info!("📥 Received lobby snapshot: {}", snapshot.name);
        tracing::info!("   Host: {}", snapshot.host_id);
        tracing::info!("   Participants: {}", snapshot.participants.len());

        // Create lobby from snapshot
        let create_lobby_cmd = DomainCommand::CreateLobby {
            lobby_id: Some(snapshot.lobby_id),
            lobby_name: snapshot.name,
            host_name: snapshot
                .participants
                .iter()
                .find(|p| p.is_host())
                .map(|p| p.name().to_string())
                .unwrap_or_else(|| "Host".to_string()),
        };

        self.pending_domain_commands.push_back(create_lobby_cmd);

        // Translate subsequent events to commands
        for event in events {
            if let Some(cmd) = self.translator.to_domain_command(&event.event) {
                self.pending_domain_commands.push_back(cmd);
            }
        }

        tracing::info!(
            "✅ Queued {} commands from snapshot",
            self.pending_domain_commands.len()
        );
    }

    /// Submit a domain event to broadcast (HOST ONLY)
    pub fn broadcast_event(&mut self, event: P2PDomainEvent) -> Result<u64> {
        let sync_msg = self
            .event_sync
            .create_event(event)
            .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))?;

        match sync_msg {
            SyncMessage::EventBroadcast { event } => {
                let sequence = event.sequence;

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

    /// Apply a core domain event and broadcast it via P2P
    pub fn apply_domain_event(&mut self, event: CoreDomainEvent) -> Result<()> {
        if let Some(p2p_event) = self.translator.to_p2p_event(event) {
            self.broadcast_event(p2p_event)?;
            tracing::debug!("✅ Applied and broadcast core domain event");
        } else {
            tracing::debug!("ℹ️  Core event not translated (likely CommandFailed)");
        }
        Ok(())
    }

    /// Process network events and send queued messages
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

                    if let Ok(sync_msg) = serde_json::from_slice::<SyncMessage>(data) {
                        tracing::debug!("📥 Received sync message from {}", from);

                        match self.event_sync.handle_message(*from, sync_msg) {
                            Ok(SyncResponse::ApplyEvents { events }) => {
                                tracing::info!("✅ Applying {} events from sync", events.len());
                                self.inbound_lobby_events.extend(events);
                            }
                            Ok(SyncResponse::SendMessage { to, message }) => {
                                if let Ok(data) = serde_json::to_vec(&message) {
                                    if let Some(peer) = to {
                                        tracing::info!("📤 Sending sync response to {}", peer);
                                        let _ = self.connection.send_to(PeerId(peer.inner()), data);
                                    } else {
                                        tracing::info!("📤 Broadcasting sync response");
                                        let _ = self.connection.broadcast(data);
                                    }
                                }
                            }
                            Ok(SyncResponse::ApplySnapshot { snapshot, events }) => {
                                tracing::info!("✅ Applying snapshot + {} events", events.len());
                                self.apply_snapshot_to_domain(snapshot, events);
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
                            }
                            Ok(SyncResponse::None) => {}
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

        // 3. Translate incoming P2P lobby events to domain commands
        let lobby_events = std::mem::take(&mut self.inbound_lobby_events);
        for lobby_event in lobby_events {
            if let Some(cmd) = self.translator.to_domain_command(&lobby_event.event) {
                tracing::debug!(
                    "🔄 Translated P2P event (seq {}) → Domain command",
                    lobby_event.sequence
                );
                self.pending_domain_commands.push_back(cmd);
            }
            self.inbound_lobby_events.push(lobby_event);
        }

        // 4. Send outbound messages (up to batch_size)
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

    /// Drain parsed lobby events
    pub fn drain_lobby_events(&mut self) -> Vec<LobbyEvent> {
        std::mem::take(&mut self.inbound_lobby_events)
    }

    /// Drain domain commands translated from incoming P2P events
    pub fn drain_domain_commands(&mut self) -> Vec<DomainCommand> {
        self.pending_domain_commands.drain(..).collect()
    }

    pub fn peer_registry(&self) -> &PeerRegistry {
        &self.peer_registry
    }

    pub fn peer_registry_mut(&mut self) -> &mut PeerRegistry {
        &mut self.peer_registry
    }

    pub fn local_peer_id(&self) -> Option<PeerId> {
        self.connection.local_peer_id()
    }

    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.connection.connected_peers()
    }

    pub fn pending_messages(&self) -> usize {
        self.outbound.len()
    }

    pub fn pending_events(&self) -> usize {
        self.inbound_events.len()
    }

    pub fn pending_domain_commands(&self) -> usize {
        self.pending_domain_commands.len()
    }

    pub fn current_sequence(&self) -> u64 {
        self.event_sync.current_sequence()
    }

    pub fn promote_to_host(&mut self) {
        self.event_sync.promote_to_host();
        tracing::info!("👑 Promoted to HOST in P2P layer");
    }

    /// Send full sync to a specific peer (HOST ONLY)
    pub fn send_full_sync_to_peer(
        &mut self,
        peer_id: PeerId,
        snapshot: LobbySnapshot,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_core::Participant;

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

    // ===== INTEGRATION TESTS: P2P ↔ Core =====

    #[test]
    fn test_domain_event_to_p2p_translation() {
        let lobby_id = Uuid::new_v4();

        // Create translator
        let translator = EventTranslator::new(lobby_id);

        // Core domain emits event
        let participant = Participant::new_guest("Alice".to_string()).unwrap();
        let core_event = CoreDomainEvent::GuestJoined {
            lobby_id,
            participant: participant.clone(),
        };

        // Translate to P2P
        let p2p_event = translator.to_p2p_event(core_event);

        assert!(p2p_event.is_some());
        match p2p_event.unwrap() {
            P2PDomainEvent::GuestJoined { participant: p } => {
                assert_eq!(p.name(), "Alice");
            }
            _ => panic!("Expected GuestJoined"),
        }
    }

    #[test]
    fn test_p2p_event_to_domain_command_translation() {
        let lobby_id = Uuid::new_v4();

        // Create translator
        let translator = EventTranslator::new(lobby_id);

        // P2P receives event
        let participant = Participant::new_guest("Bob".to_string()).unwrap();
        let p2p_event = P2PDomainEvent::GuestJoined {
            participant: participant.clone(),
        };

        // Translate to domain command
        let cmd = translator.to_domain_command(&p2p_event);

        assert!(cmd.is_some());
        match cmd.unwrap() {
            DomainCommand::JoinLobby {
                lobby_id: lid,
                guest_name,
            } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(guest_name, "Bob");
            }
            _ => panic!("Expected JoinLobby command"),
        }
    }

    #[test]
    fn test_roundtrip_translation() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        // Start with core event
        let participant = Participant::new_guest("Charlie".to_string()).unwrap();
        let original_core = CoreDomainEvent::GuestJoined {
            lobby_id,
            participant: participant.clone(),
        };

        // Core → P2P
        let p2p_event = translator
            .to_p2p_event(original_core.clone())
            .expect("Should translate to P2P");

        // P2P → Domain Command
        let domain_cmd = translator
            .to_domain_command(&p2p_event)
            .expect("Should translate to command");

        // Verify command correctness
        match domain_cmd {
            DomainCommand::JoinLobby {
                lobby_id: lid,
                guest_name,
            } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(guest_name, "Charlie");
            }
            _ => panic!("Expected JoinLobby command"),
        }
    }

    #[test]
    fn test_command_failed_not_translated() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        // CommandFailed should not be broadcast
        let core_event = CoreDomainEvent::CommandFailed {
            command: "Test".to_string(),
            reason: "Error".to_string(),
        };

        let p2p_event = translator.to_p2p_event(core_event);
        assert!(p2p_event.is_none());
    }

    #[test]
    fn test_host_delegated_translation() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let from = Uuid::new_v4();
        let to = Uuid::new_v4();

        // Core → P2P
        let core_event = CoreDomainEvent::HostDelegated { lobby_id, from, to };

        let p2p_event = translator
            .to_p2p_event(core_event)
            .expect("Should translate");

        match p2p_event {
            P2PDomainEvent::HostDelegated {
                from: f,
                to: t,
                reason: _,
            } => {
                assert_eq!(f, from);
                assert_eq!(t, to);
            }
            _ => panic!("Expected HostDelegated"),
        }

        // P2P → Domain Command
        let p2p_event_for_cmd = P2PDomainEvent::HostDelegated {
            from,
            to,
            reason: crate::domain::DelegationReason::Manual,
        };

        let cmd = translator
            .to_domain_command(&p2p_event_for_cmd)
            .expect("Should translate");

        match cmd {
            DomainCommand::DelegateHost {
                lobby_id: lid,
                current_host_id,
                new_host_id,
            } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(current_host_id, from);
                assert_eq!(new_host_id, to);
            }
            _ => panic!("Expected DelegateHost command"),
        }
    }

    #[test]
    fn test_participation_mode_changed_translation() {
        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);
        let participant_id = Uuid::new_v4();

        // Core → P2P
        let core_event = CoreDomainEvent::ParticipationModeChanged {
            lobby_id,
            participant_id,
            new_mode: konnekt_session_core::ParticipationMode::Spectating,
        };

        let p2p_event = translator
            .to_p2p_event(core_event)
            .expect("Should translate");

        match p2p_event {
            P2PDomainEvent::ParticipationModeChanged {
                participant_id: pid,
                new_mode,
            } => {
                assert_eq!(pid, participant_id);
                assert_eq!(new_mode, "Spectating");
            }
            _ => panic!("Expected ParticipationModeChanged"),
        }

        // P2P → Domain Command
        let p2p_event_for_cmd = P2PDomainEvent::ParticipationModeChanged {
            participant_id,
            new_mode: "Active".to_string(),
        };

        let cmd = translator
            .to_domain_command(&p2p_event_for_cmd)
            .expect("Should translate");

        match cmd {
            DomainCommand::ToggleParticipationMode {
                lobby_id: lid,
                participant_id: pid,
                requester_id: rid,
                activity_in_progress,
            } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(pid, participant_id);
                assert_eq!(rid, participant_id); // Self-toggle
                assert!(!activity_in_progress); // Default
            }
            _ => panic!("Expected ToggleParticipationMode command"),
        }
    }

    #[test]
    fn test_peer_timeout_creates_domain_command() {
        // This would require a mock connection to simulate timeout
        // For now, we test the translation logic separately

        let lobby_id = Uuid::new_v4();
        let translator = EventTranslator::new(lobby_id);

        let participant_id = Uuid::new_v4();

        // Simulate peer timeout → participant leaves
        let p2p_event = P2PDomainEvent::GuestLeft { participant_id };

        let cmd = translator
            .to_domain_command(&p2p_event)
            .expect("Should translate");

        match cmd {
            DomainCommand::LeaveLobby {
                lobby_id: lid,
                participant_id: pid,
            } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(pid, participant_id);
            }
            _ => panic!("Expected LeaveLobby command"),
        }
    }
}
