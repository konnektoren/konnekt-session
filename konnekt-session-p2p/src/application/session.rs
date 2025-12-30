use crate::application::sync_manager::{
    EventSyncManager, LobbySnapshot, SyncMessage, SyncResponse,
};
use crate::application::{ConnectionEvent, SessionConfig};
use crate::domain::{DomainEvent, LobbyEvent, PeerId, PeerRegistry, SessionId};
use crate::infrastructure::{connection::MatchboxConnection, error::Result};
use instant::Duration;
use uuid::Uuid;

/// Application service: High-level P2P session management
pub struct P2PSession {
    session_id: SessionId,
    connection: MatchboxConnection,
    /// Registry tracking all connected peers and their state
    peer_registry: PeerRegistry,
    /// Event synchronization manager
    event_sync: Option<EventSyncManager>,
}

impl P2PSession {
    /// Create a new P2P session (as host) with default config
    pub async fn create_host(signalling_server: &str) -> Result<Self> {
        let config = SessionConfig::new(signalling_server.to_string());
        Self::create_host_with_config(config).await
    }

    /// Create a new P2P session (as host) with custom config
    pub async fn create_host_with_config(config: SessionConfig) -> Result<Self> {
        let session_id = SessionId::new();
        Self::join_with_config(config, session_id).await
    }

    /// Join an existing P2P session (as guest) with default config
    pub async fn join(signalling_server: &str, session_id: SessionId) -> Result<Self> {
        let config = SessionConfig::new(signalling_server.to_string());
        Self::join_with_config(config, session_id).await
    }

    /// Join an existing P2P session (as guest) with custom config
    pub async fn join_with_config(config: SessionConfig, session_id: SessionId) -> Result<Self> {
        let room_url = format!("{}/{}", config.signalling_server, session_id.as_str());

        tracing::info!("Joining session {} at {}", session_id, room_url);
        tracing::debug!("Using {} ICE servers", config.ice_servers.len());

        let connection = MatchboxConnection::connect(&room_url, config.ice_servers).await?;

        Ok(P2PSession {
            session_id,
            connection,
            peer_registry: PeerRegistry::with_grace_period(Duration::from_secs(30)),
            event_sync: None, // Initialize as None - will be set via init_sync_*
        })
    }

    /// Initialize event sync as host
    pub fn init_sync_as_host(&mut self, lobby_id: Uuid) {
        self.event_sync = Some(EventSyncManager::new_host(lobby_id));
        tracing::info!("Initialized event sync as host for lobby {}", lobby_id);
    }

    /// Initialize event sync as guest
    pub fn init_sync_as_guest(&mut self, lobby_id: Uuid) {
        self.event_sync = Some(EventSyncManager::new_guest(lobby_id));
        tracing::info!("Initialized event sync as guest for lobby {}", lobby_id);
    }

    /// Create a domain event (host only)
    pub fn create_event(&mut self, event: DomainEvent) -> Result<()> {
        let sync = self.event_sync.as_mut().ok_or_else(|| {
            crate::infrastructure::error::P2PError::ConnectionFailed(
                "Event sync not initialized".to_string(),
            )
        })?;

        match sync.create_event(event) {
            Ok(SyncMessage::EventBroadcast { event }) => {
                // Serialize and broadcast
                let data = serde_json::to_vec(&SyncMessage::EventBroadcast { event })
                    .map_err(|e| crate::infrastructure::error::P2PError::Serialization(e))?;

                self.connection.broadcast(data)?;
                Ok(())
            }
            Ok(_) => Ok(()),
            Err(e) => Err(crate::infrastructure::error::P2PError::ConnectionFailed(
                format!("Failed to create event: {:?}", e),
            )),
        }
    }

    /// Send full sync to a peer (host only)
    pub fn send_full_sync(&mut self, to_peer: PeerId, snapshot: LobbySnapshot) -> Result<()> {
        let sync = self.event_sync.as_ref().ok_or_else(|| {
            crate::infrastructure::error::P2PError::ConnectionFailed(
                "Event sync not initialized".to_string(),
            )
        })?;

        match sync.create_full_sync_response(0, snapshot) {
            Ok(message) => {
                let data = serde_json::to_vec(&message)
                    .map_err(|e| crate::infrastructure::error::P2PError::Serialization(e))?;

                self.connection.send_to(to_peer, data)?;
                Ok(())
            }
            Err(e) => Err(crate::infrastructure::error::P2PError::ConnectionFailed(
                format!("Failed to create full sync: {:?}", e),
            )),
        }
    }

    /// Get the session ID
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// Get our local peer ID
    pub fn local_peer_id(&self) -> Option<PeerId> {
        self.connection.local_peer_id()
    }

    /// Get list of connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.connection.connected_peers()
    }

    /// Send data to a specific peer
    pub fn send_to(&mut self, peer: PeerId, data: Vec<u8>) -> Result<()> {
        self.connection.send_to(peer, data)
    }

    /// Broadcast data to all peers
    pub fn broadcast(&mut self, data: Vec<u8>) -> Result<()> {
        self.connection.broadcast(data)
    }

    /// Register participant information for a peer
    pub fn register_peer_participant(
        &mut self,
        peer_id: PeerId,
        participant_id: Uuid,
        name: String,
        is_host: bool,
    ) {
        if let Some(peer_state) = self.peer_registry.get_peer_mut(&peer_id) {
            peer_state.set_participant_info(participant_id, name, is_host);
            tracing::debug!(
                "Registered participant {} for peer {} (host: {})",
                participant_id,
                peer_id,
                is_host
            );
        }
    }

    /// Find participant ID for a given peer ID
    pub fn find_participant_by_peer(&self, peer_id: &PeerId) -> Option<uuid::Uuid> {
        self.peer_registry
            .get_peer(peer_id)
            .and_then(|state| state.participant_id)
    }

    /// Find peer ID by participant UUID
    pub fn find_peer_by_participant(&self, participant_id: Uuid) -> Option<PeerId> {
        self.peer_registry.find_by_participant_id(participant_id)
    }

    /// Check if a peer is the host
    pub fn is_peer_host(&self, peer_id: &PeerId) -> bool {
        self.peer_registry.is_peer_host(peer_id)
    }

    /// Find the host peer
    pub fn find_host_peer(&self) -> Option<PeerId> {
        self.peer_registry.find_host().map(|(peer_id, _)| peer_id)
    }

    /// Poll for connection events
    pub fn poll_events(&mut self) -> Vec<ConnectionEvent> {
        let mut events = self.connection.poll_events();
        let mut sync_events_to_apply = Vec::new();

        // Update peer registry based on connection events
        for event in &events {
            match event {
                ConnectionEvent::PeerConnected(peer) => {
                    self.peer_registry.add_peer(*peer);
                    tracing::debug!("Added peer {} to registry", peer);
                }
                ConnectionEvent::MessageReceived { from, data } => {
                    self.peer_registry.update_last_seen(from);

                    // Try to parse as SyncMessage
                    if let Some(sync) = &mut self.event_sync {
                        if let Ok(msg) = serde_json::from_slice::<SyncMessage>(data) {
                            tracing::debug!("Received sync message from {}", from);

                            match sync.handle_message(*from, msg) {
                                Ok(SyncResponse::ApplyEvents {
                                    events: lobby_events,
                                }) => {
                                    tracing::info!(
                                        "Applying {} events from sync",
                                        lobby_events.len()
                                    );
                                    sync_events_to_apply.extend(lobby_events);
                                }
                                Ok(SyncResponse::SendMessage { to, message }) => {
                                    // Send response
                                    if let Ok(data) = serde_json::to_vec(&message) {
                                        if let Some(peer) = to {
                                            let _ = self.connection.send_to(peer, data);
                                        } else {
                                            let _ = self.connection.broadcast(data);
                                        }
                                    }
                                }
                                Ok(SyncResponse::ApplySnapshot {
                                    snapshot: _,
                                    events: lobby_events,
                                }) => {
                                    tracing::info!(
                                        "Applying snapshot + {} events",
                                        lobby_events.len()
                                    );
                                    // For now, just apply the events
                                    sync_events_to_apply.extend(lobby_events);
                                }
                                Ok(SyncResponse::NeedSnapshot {
                                    for_peer,
                                    since_sequence,
                                }) => {
                                    tracing::info!(
                                        "Peer {} needs snapshot from sequence {}",
                                        for_peer,
                                        since_sequence
                                    );
                                    // Application layer will need to provide snapshot
                                }
                                Ok(SyncResponse::None) => {
                                    // No action needed
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to handle sync message: {:?}", e);
                                }
                            }
                        }
                    }
                }
                ConnectionEvent::PeerDisconnected(peer) => {
                    // Don't remove immediately - start grace period
                    self.peer_registry.mark_peer_disconnected(peer);
                    tracing::debug!(
                        "Marked peer {} as disconnected (grace period started)",
                        peer
                    );
                }
                _ => {}
            }
        }

        // Expose sync events as special ConnectionEvent messages
        if !sync_events_to_apply.is_empty() {
            for lobby_event in sync_events_to_apply {
                // Serialize the LobbyEvent back to JSON
                if let Ok(data) = serde_json::to_vec(&lobby_event) {
                    events.push(ConnectionEvent::MessageReceived {
                        from: self.local_peer_id().unwrap_or_else(|| {
                            PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()))
                        }),
                        data,
                    });
                }
            }
        }

        // Check for grace period timeouts
        let timed_out_peers = self.peer_registry.check_grace_periods();

        for peer_id in timed_out_peers {
            if let Some(peer_state) = self.peer_registry.get_peer(&peer_id) {
                let participant_id = peer_state.participant_id;
                let was_host = peer_state.is_host;

                tracing::warn!(
                    "Peer {} timed out after grace period (was_host: {})",
                    peer_id,
                    was_host
                );

                events.push(ConnectionEvent::PeerTimedOut {
                    peer_id,
                    participant_id,
                    was_host,
                });

                // Now remove from registry
                self.peer_registry.remove_peer(&peer_id);
            }
        }

        events
    }
}
