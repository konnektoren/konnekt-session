use crate::application::{ConnectionEvent, SessionConfig};
use crate::domain::{PeerId, PeerRegistry, SessionId};
use crate::infrastructure::{connection::MatchboxConnection, error::Result};
use instant::Duration;
use uuid::Uuid;

/// Application service: High-level P2P session management
pub struct P2PSession {
    session_id: SessionId,
    connection: MatchboxConnection,
    /// Registry tracking all connected peers and their state
    peer_registry: PeerRegistry,
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
        })
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

        // Update peer registry based on connection events
        for event in &events {
            match event {
                ConnectionEvent::PeerConnected(peer) => {
                    self.peer_registry.add_peer(*peer);
                    tracing::debug!("Added peer {} to registry", peer);
                }
                ConnectionEvent::MessageReceived { from, .. } => {
                    self.peer_registry.update_last_seen(from);
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
