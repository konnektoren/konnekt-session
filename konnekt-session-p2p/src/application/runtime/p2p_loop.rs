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

    /// Send a domain command to host (GUEST ONLY)
    pub fn send_command_to_host(&mut self, command: DomainCommand) -> Result<()> {
        tracing::info!("📤 GUEST: Sending command to host: {:?}", command);

        let msg = SyncMessage::CommandRequest { command };
        let data = serde_json::to_vec(&msg)
            .map_err(|e| crate::infrastructure::error::P2PError::Serialization(e))?;

        self.connection.broadcast(data)?;
        Ok(())
    }

    /// Request full sync from host (GUEST ONLY)
    pub fn request_full_sync(&mut self) -> Result<()> {
        let sync_msg = self
            .event_sync
            .request_full_sync()
            .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))?;

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

    /// Broadcast a domain event (HOST ONLY)
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

        tracing::debug!("✅ Broadcast domain event via P2P");
        Ok(())
    }

    /// Process network events
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
                            Ok(SyncResponse::ProcessCommand { command }) => {
                                tracing::info!("📥 HOST: Processing command from peer {}", from);
                                self.pending_domain_commands.push_back(command);
                            }
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
                                // Will be handled by SessionLoop
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

        // 3. Translate incoming lobby events to domain commands
        let lobby_events = std::mem::take(&mut self.inbound_lobby_events);
        for lobby_event in lobby_events {
            if let Some(cmd) = self.translator.to_domain_command(&lobby_event.event) {
                tracing::debug!(
                    "🔄 Translated P2P event (seq {}) → Domain command",
                    lobby_event.sequence
                );
                self.pending_domain_commands.push_back(cmd);
            }
        }

        processed
    }

    /// Drain connection events
    pub fn drain_events(&mut self) -> Vec<ConnectionEvent> {
        std::mem::take(&mut self.inbound_events)
    }

    /// Drain domain commands
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

    pub fn pending_messages(&self) -> usize {
        self.outbound.len()
    }

    pub fn pending_domain_commands(&self) -> usize {
        self.pending_domain_commands.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_routing() {
        // This would require mocking the connection
        // For now, verify API exists
        let cmd = DomainCommand::JoinLobby {
            lobby_id: Uuid::new_v4(),
            guest_name: "Test".to_string(),
        };

        // Just verify it compiles
        let _ = format!("{:?}", cmd);
    }
}
