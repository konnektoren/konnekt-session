use crate::application::runtime::MessageQueue;
use crate::application::sync_manager::{EventSyncManager, SyncMessage, SyncResponse};
use crate::application::{ConnectionEvent, EventTranslator, LobbySnapshot};
use crate::domain::{DomainEvent as P2PDomainEvent, LobbyEvent, PeerId, PeerRegistry};
use crate::infrastructure::connection::MatchboxConnection;
use crate::infrastructure::error::Result;
use instant::Duration;
use konnekt_session_core::{DomainCommand, DomainEvent as CoreDomainEvent};
use std::collections::VecDeque;
use uuid::Uuid;

// 🆕 Add tracing
use tracing::{debug, info, instrument, trace, warn};

/// P2P event loop - handles network communication and event ordering
pub struct P2PLoop {
    /// WebRTC connection (Matchbox adapter)
    connection: MatchboxConnection,

    /// Peer registry (tracks connection state)
    peer_registry: PeerRegistry,

    /// Event synchronization manager
    event_sync: EventSyncManager,

    /// Event translator (P2P ↔ Core domain)
    translator: EventTranslator,

    /// Outbound message queue
    outbound: MessageQueue,

    /// Inbound connection events
    inbound_events: Vec<ConnectionEvent>,

    /// Inbound lobby events
    inbound_lobby_events: Vec<LobbyEvent>,

    /// Domain commands to be processed by SessionLoop
    pending_domain_commands: VecDeque<DomainCommand>,

    /// Max events to process per poll
    batch_size: usize,
}

impl P2PLoop {
    /// Create a new P2P loop as HOST
    #[instrument(skip(connection), fields(lobby_id = %lobby_id))]
    pub fn new_host(
        connection: MatchboxConnection,
        lobby_id: Uuid,
        batch_size: usize,
        max_queue_size: usize,
    ) -> Self {
        info!("P2PLoop initialized as HOST");
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
    #[instrument(skip(connection), fields(lobby_id = %lobby_id))]
    pub fn new_guest(
        connection: MatchboxConnection,
        lobby_id: Uuid,
        batch_size: usize,
        max_queue_size: usize,
    ) -> Self {
        info!("P2PLoop initialized as GUEST");
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

    /// Send a domain command to host (GUEST ONLY)
    #[instrument(skip(self), fields(command_type = ?std::mem::discriminant(&command)))]
    pub fn send_command_to_host(&mut self, command: DomainCommand) -> Result<()> {
        debug!("GUEST: Sending command to host");

        let msg = SyncMessage::CommandRequest { command };
        let data = serde_json::to_vec(&msg)
            .map_err(|e| crate::infrastructure::error::P2PError::Serialization(e))?;

        self.connection.broadcast(data)?;
        trace!("Command broadcast complete");
        Ok(())
    }

    /// Request full sync from host (GUEST ONLY)
    #[instrument(skip(self))]
    pub fn request_full_sync(&mut self) -> Result<()> {
        let sync_msg = self
            .event_sync
            .request_full_sync()
            .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))?;

        let data = serde_json::to_vec(&sync_msg)
            .map_err(|e| crate::infrastructure::error::P2PError::Serialization(e))?;

        self.connection.broadcast(data)?;

        info!("Sent full sync request to host");
        Ok(())
    }

    /// Apply snapshot to domain layer (converts snapshot to domain commands)
    #[instrument(skip(self, snapshot, events), fields(
        snapshot.lobby_id = %snapshot.lobby_id,
        snapshot.name = %snapshot.name,
        participants = %snapshot.participants.len(),
        events_count = %events.len()
    ))]
    fn apply_snapshot_to_domain(&mut self, snapshot: LobbySnapshot, events: Vec<LobbyEvent>) {
        info!("Applying lobby snapshot");
        debug!(host_id = %snapshot.host_id, "Snapshot host");

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

        info!(commands_queued = %self.pending_domain_commands.len(), "Snapshot applied");
    }

    /// Broadcast a domain event (HOST ONLY)
    #[instrument(skip(self, event), fields(event_type = ?std::mem::discriminant(&event)))]
    pub fn broadcast_domain_event(&mut self, event: CoreDomainEvent) -> Result<()> {
        // Translate core event to P2P event
        let p2p_event = self.translator.to_p2p_event(event).ok_or_else(|| {
            crate::infrastructure::error::P2PError::SendFailed(
                "Event not translatable to P2P".to_string(),
            )
        })?;

        // Create sequenced lobby event
        let sync_msg = self
            .event_sync
            .create_event(p2p_event)
            .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))?;

        // Serialize and broadcast
        let data = serde_json::to_vec(&sync_msg)
            .map_err(|e| crate::infrastructure::error::P2PError::Serialization(e))?;

        self.connection.broadcast(data)?;

        trace!("Domain event broadcast complete");
        Ok(())
    }

    /// Process network events
    #[instrument(skip(self), fields(peer_count = %self.connection.connected_peers().len()))]
    pub fn poll(&mut self) -> usize {
        let mut processed = 0;

        // 1. Poll connection for network events
        let connection_events = self.connection.poll_events();

        for event in connection_events {
            processed += 1;

            match &event {
                ConnectionEvent::PeerConnected(peer_id) => {
                    self.peer_registry.add_peer(*peer_id);
                    debug!(peer_id = %peer_id, "Added peer to registry");
                }
                ConnectionEvent::MessageReceived { from, data } => {
                    self.peer_registry.update_last_seen(from);
                    trace!(peer_id = %from, bytes = %data.len(), "Received message");

                    if let Ok(sync_msg) = serde_json::from_slice::<SyncMessage>(data) {
                        debug!(peer_id = %from, "Received sync message");

                        match self.event_sync.handle_message(*from, sync_msg) {
                            Ok(SyncResponse::ProcessCommand { command }) => {
                                info!(peer_id = %from, "HOST: Processing command from peer");
                                self.pending_domain_commands.push_back(command);
                            }
                            Ok(SyncResponse::ApplyEvents { events }) => {
                                info!(events = %events.len(), "Applying events from sync");
                                self.inbound_lobby_events.extend(events);
                            }
                            Ok(SyncResponse::SendMessage { to, message }) => {
                                if let Ok(data) = serde_json::to_vec(&message) {
                                    if let Some(peer) = to {
                                        debug!(peer_id = %peer, "Sending sync response");
                                        let _ = self.connection.send_to(PeerId(peer.inner()), data);
                                    } else {
                                        debug!("Broadcasting sync response");
                                        let _ = self.connection.broadcast(data);
                                    }
                                }
                            }
                            Ok(SyncResponse::ApplySnapshot { snapshot, events }) => {
                                info!(events = %events.len(), "Applying snapshot");
                                self.apply_snapshot_to_domain(snapshot, events);
                            }
                            Ok(SyncResponse::NeedSnapshot {
                                for_peer,
                                since_sequence,
                            }) => {
                                info!(
                                    peer_id = %for_peer,
                                    since_sequence = %since_sequence,
                                    "Peer needs snapshot"
                                );
                            }
                            Ok(SyncResponse::None) => {
                                trace!("Sync message processed (no action)");
                            }
                            Err(e) => {
                                warn!(error = ?e, "Failed to handle sync message");
                            }
                        }
                    }
                }
                ConnectionEvent::PeerDisconnected(peer_id) => {
                    self.peer_registry.mark_peer_disconnected(peer_id);
                    debug!(peer_id = %peer_id, "Marked peer as disconnected");
                }
                ConnectionEvent::PeerTimedOut { peer_id, .. } => {
                    self.peer_registry.remove_peer(peer_id);
                    debug!(peer_id = %peer_id, "Removed peer after timeout");
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

                warn!(peer_id = %peer_id, was_host = %was_host, "Peer timed out");

                self.inbound_events.push(ConnectionEvent::PeerTimedOut {
                    peer_id,
                    participant_id,
                    was_host,
                });

                processed += 1;
            }

            self.peer_registry.remove_peer(&peer_id);
        }

        // 3. Translate incoming lobby events to domain commands
        let lobby_events = std::mem::take(&mut self.inbound_lobby_events);
        for lobby_event in lobby_events {
            if let Some(cmd) = self.translator.to_domain_command(&lobby_event.event) {
                trace!(sequence = %lobby_event.sequence, "Translated P2P event → Domain command");
                self.pending_domain_commands.push_back(cmd);
            }
        }

        if processed > 0 {
            debug!(processed = %processed, "Poll cycle complete");
        }

        processed
    }

    /// Send full sync to a specific peer (HOST ONLY)
    #[instrument(skip(self, snapshot), fields(
        peer_id = %peer_id,
        snapshot.lobby_id = %snapshot.lobby_id,
        participants = %snapshot.participants.len()
    ))]
    pub fn send_full_sync_to_peer(
        &mut self,
        peer_id: PeerId,
        snapshot: LobbySnapshot,
    ) -> Result<()> {
        info!("Sending full sync to peer");

        let sync_msg = self
            .event_sync
            .create_full_sync_response(0, snapshot)
            .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))?;

        let data = serde_json::to_vec(&sync_msg)
            .map_err(|e| crate::infrastructure::error::P2PError::Serialization(e))?;

        self.connection.send_to(PeerId(peer_id.inner()), data)?;

        debug!("Full sync sent successfully");
        Ok(())
    }

    // ... rest of methods unchanged ...

    pub fn drain_events(&mut self) -> Vec<ConnectionEvent> {
        std::mem::take(&mut self.inbound_events)
    }

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

    pub fn current_sequence(&self) -> u64 {
        self.event_sync.current_sequence()
    }

    #[instrument(skip(self))]
    pub fn promote_to_host(&mut self) {
        info!("Promoting to HOST in P2P layer");
        self.event_sync.promote_to_host();
    }

    pub fn pending_messages(&self) -> usize {
        self.outbound.len()
    }

    pub fn pending_domain_commands(&self) -> usize {
        self.pending_domain_commands.len()
    }
}
