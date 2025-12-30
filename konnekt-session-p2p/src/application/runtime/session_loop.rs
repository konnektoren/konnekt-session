use crate::application::LobbySnapshot;
use crate::application::runtime::P2PLoop;
use crate::domain::PeerId;
use crate::infrastructure::error::Result;
use konnekt_session_core::{DomainCommand, DomainEvent as CoreDomainEvent, DomainLoop, Lobby};
use uuid::Uuid;

/// Unified session loop that coordinates P2P ↔ Core
///
/// This is the single integration point between networking and business logic.
pub struct SessionLoop {
    /// P2P networking layer
    p2p: P2PLoop,

    /// Core domain layer
    domain: DomainLoop,

    /// Lobby ID (1:1 with session)
    lobby_id: Uuid,

    /// Are we the host?
    is_host: bool,
}

impl SessionLoop {
    /// Create a new session loop for HOST
    pub fn new_host(p2p: P2PLoop, domain: DomainLoop, lobby_id: Uuid) -> Self {
        tracing::info!("🎯 SessionLoop created as HOST for lobby {}", lobby_id);

        Self {
            p2p,
            domain,
            lobby_id,
            is_host: true,
        }
    }

    /// Create a new session loop for GUEST
    pub fn new_guest(mut p2p: P2PLoop, domain: DomainLoop, lobby_id: Uuid) -> Self {
        tracing::info!("🎯 SessionLoop created as GUEST for lobby {}", lobby_id);

        // Guest immediately requests full sync from host
        tracing::info!("🔄 Guest auto-requesting full sync from host");

        if let Err(e) = p2p.request_full_sync() {
            tracing::warn!("Failed to request full sync: {:?}", e);
        }

        Self {
            p2p,
            domain,
            lobby_id,
            is_host: false,
        }
    }

    /// Submit a domain command
    ///
    /// - Host: Processes locally
    /// - Guest: Sends to host via P2P
    pub fn submit_command(&mut self, cmd: DomainCommand) -> Result<()> {
        tracing::debug!("📝 Submitting domain command: {:?}", cmd);

        if self.is_host {
            // Host: Process locally
            self.domain
                .submit(cmd)
                .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))
        } else {
            // Guest: Send to host via P2P
            self.p2p.send_command_to_host(cmd)
        }
    }

    /// Register participant with peer (for tracking disconnections)
    fn register_participant_for_peer(&mut self, participant_id: Uuid) {
        if let Some(peer_id) = self.local_peer_id() {
            if let Some(state) = self.p2p.peer_registry_mut().get_peer_mut(&peer_id) {
                state.set_participant_info(participant_id, String::new(), self.is_host);

                tracing::debug!(
                    "📝 Registered participant {} for peer {}",
                    participant_id,
                    peer_id
                );
            }
        }
    }

    /// Map the most recent unregistered peer to a participant
    /// Call this after GuestJoined event
    fn map_newest_guest_to_participant(&mut self, participant_id: Uuid, participant_name: &str) {
        // Find connected peers without participant IDs
        let unregistered_peers: Vec<PeerId> = self
            .p2p
            .connected_peers()
            .into_iter()
            .filter(|peer_id| {
                self.p2p
                    .peer_registry()
                    .get_peer(peer_id)
                    .and_then(|state| state.participant_id)
                    .is_none()
            })
            .collect();

        if let Some(peer_id) = unregistered_peers.first() {
            tracing::info!(
                "📝 HOST: Registering peer {} as participant {} ({})",
                peer_id,
                participant_id,
                participant_name
            );

            if let Some(state) = self.p2p.peer_registry_mut().get_peer_mut(peer_id) {
                state.set_participant_info(participant_id, participant_name.to_string(), false);
            }
        } else {
            tracing::warn!(
                "⚠️  No unregistered peer found for participant {}",
                participant_id
            );
        }
    }

    /// Main event loop - call this regularly (e.g., every 100ms)
    ///
    /// This AUTOMATICALLY:
    /// 1. Polls P2P for network events
    /// 2. Gets domain commands (from P2P or translated events)
    /// 3. Processes commands in domain
    /// 4. Broadcasts resulting events (HOST ONLY)
    pub fn poll(&mut self) -> usize {
        let mut processed = 0;

        // ===== Step 1: Poll P2P (network events) =====
        let p2p_processed = self.p2p.poll();
        processed += p2p_processed;

        if p2p_processed > 0 {
            tracing::trace!("📡 P2P processed {} events", p2p_processed);
        }

        // ===== Step 1.5: Handle connection events =====
        if self.is_host {
            let connection_events = self.p2p.drain_events();

            for event in &connection_events {
                match event {
                    crate::application::ConnectionEvent::PeerConnected(peer_id) => {
                        tracing::info!(
                            "🟢 HOST: Peer {} connected - auto-sending full sync",
                            peer_id
                        );

                        if let Some(lobby) = self.get_lobby() {
                            let snapshot = LobbySnapshot {
                                lobby_id: lobby.id(),
                                name: lobby.name().to_string(),
                                host_id: lobby.host_id(),
                                participants: lobby.participants().values().cloned().collect(),
                                as_of_sequence: self.p2p.current_sequence(),
                            };

                            if let Err(e) = self.p2p.send_full_sync_to_peer(*peer_id, snapshot) {
                                tracing::error!(
                                    "❌ Failed to send full sync to {}: {}",
                                    peer_id,
                                    e
                                );
                            } else {
                                tracing::info!("✅ Sent full sync to {}", peer_id);
                            }
                        } else {
                            tracing::warn!("⚠️  No lobby to sync to peer {}", peer_id);
                        }
                    }

                    // 🆕 Handle peer timeout → remove participant from lobby
                    crate::application::ConnectionEvent::PeerTimedOut {
                        peer_id,
                        participant_id,
                        was_host,
                    } => {
                        tracing::warn!(
                            "⏰ HOST: Peer {} timed out (participant: {:?}, was_host: {})",
                            peer_id,
                            participant_id,
                            was_host
                        );

                        // If peer had a participant, remove them from lobby
                        if let Some(participant_id) = participant_id {
                            tracing::info!(
                                "🔴 HOST: Auto-removing participant {} (peer timed out)",
                                participant_id
                            );

                            // Submit LeaveLobby command
                            let leave_cmd = DomainCommand::LeaveLobby {
                                lobby_id: self.lobby_id,
                                participant_id: *participant_id,
                            };

                            if let Err(e) = self.domain.submit(leave_cmd) {
                                tracing::error!(
                                    "Failed to submit LeaveLobby for timed-out peer: {:?}",
                                    e
                                );
                            }

                            // If it was the host who timed out, we need delegation
                            if *was_host {
                                tracing::warn!("⚠️  Host timed out! Delegation needed (TODO)");
                                // TODO: Trigger host delegation
                            }
                        }
                    }

                    _ => {}
                }
            }
        } else {
            // Guest: Just drain events (don't process them)
            self.p2p.drain_events();
        }

        // ===== Step 2: Get domain commands from P2P =====
        let commands = self.p2p.drain_domain_commands();

        if !commands.is_empty() {
            tracing::info!("📥 Received {} domain commands from P2P", commands.len());
        }

        for cmd in commands {
            match &cmd {
                DomainCommand::CreateLobby { lobby_name, .. } => {
                    tracing::info!("📥 Received lobby creation: {}", lobby_name);
                }
                DomainCommand::JoinLobby { guest_name, .. } => {
                    tracing::info!("📥 Guest '{}' wants to join", guest_name);
                }
                DomainCommand::LeaveLobby { participant_id, .. } => {
                    tracing::info!("📥 Participant {} leaving", participant_id);
                }
                _ => {
                    tracing::debug!("📥 Received command: {:?}", cmd);
                }
            }

            if let Err(e) = self.domain.submit(cmd) {
                tracing::warn!("Failed to submit command to domain: {:?}", e);
            }
        }

        // ===== Step 3: Process domain commands =====
        let domain_processed = self.domain.poll();
        processed += domain_processed;

        if domain_processed > 0 {
            tracing::debug!("🔧 Domain processed {} commands", domain_processed);
        }

        // ===== Step 4: Broadcast domain events (HOST ONLY) =====
        let events = self.domain.drain_events();

        if !events.is_empty() {
            tracing::debug!("📤 Domain emitted {} events", events.len());
        }

        for event in events {
            match &event {
                CoreDomainEvent::LobbyCreated { lobby } => {
                    tracing::info!("📤 Domain event: LobbyCreated - {}", lobby.name());
                }
                CoreDomainEvent::GuestJoined { participant, .. } => {
                    tracing::info!("📤 Domain event: GuestJoined - {}", participant.name());

                    // 🆕 HOST: Register peer → participant mapping
                    if self.is_host {
                        self.map_newest_guest_to_participant(participant.id(), participant.name());
                    }

                    // Guest registers themselves
                    if !self.is_host {
                        self.register_participant_for_peer(participant.id());
                    }
                }
                CoreDomainEvent::GuestLeft { participant_id, .. } => {
                    tracing::info!("📤 Domain event: GuestLeft - {}", participant_id);
                }
                CoreDomainEvent::CommandFailed { command, reason } => {
                    tracing::warn!("⚠️  Command failed: {} - {}", command, reason);
                }
                _ => {
                    tracing::debug!("📤 Domain event: {:?}", event);
                }
            }

            // Only host broadcasts to P2P
            if self.is_host {
                // Don't broadcast failures
                if matches!(event, CoreDomainEvent::CommandFailed { .. }) {
                    continue;
                }

                if let Err(e) = self.p2p.broadcast_domain_event(event) {
                    tracing::warn!("Failed to broadcast event: {:?}", e);
                } else {
                    tracing::debug!("✅ Event broadcast to P2P network");
                }
            }
        }

        processed
    }

    /// Get the current lobby state (for rendering UI)
    pub fn get_lobby(&self) -> Option<&Lobby> {
        self.domain.event_loop().get_lobby(&self.lobby_id)
    }

    pub fn lobby_id(&self) -> Uuid {
        self.lobby_id
    }

    pub fn local_peer_id(&self) -> Option<PeerId> {
        self.p2p.local_peer_id()
    }

    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.p2p.connected_peers()
    }

    pub fn is_host(&self) -> bool {
        self.is_host
    }

    pub fn promote_to_host(&mut self) {
        tracing::info!("👑 Promoting to HOST");
        self.is_host = true;
        self.p2p.promote_to_host();
    }

    pub fn send_full_sync_to_peer(&mut self, peer_id: PeerId) -> Result<()> {
        if !self.is_host {
            return Err(crate::infrastructure::error::P2PError::SendFailed(
                "Only host can send full sync".to_string(),
            ));
        }

        tracing::info!("📤 Sending full sync to peer {}", peer_id);

        let lobby = self
            .get_lobby()
            .ok_or_else(|| {
                crate::infrastructure::error::P2PError::SendFailed("No lobby found".to_string())
            })?
            .clone();

        let snapshot = LobbySnapshot {
            lobby_id: lobby.id(),
            name: lobby.name().to_string(),
            host_id: lobby.host_id(),
            participants: lobby.participants().values().cloned().collect(),
            as_of_sequence: self.p2p.current_sequence(),
        };

        self.p2p.send_full_sync_to_peer(peer_id, snapshot)
    }

    pub fn p2p(&self) -> &P2PLoop {
        &self.p2p
    }

    pub fn p2p_mut(&mut self) -> &mut P2PLoop {
        &mut self.p2p
    }

    pub fn domain(&self) -> &DomainLoop {
        &self.domain
    }

    pub fn domain_mut(&mut self) -> &mut DomainLoop {
        &mut self.domain
    }

    pub fn current_sequence(&self) -> u64 {
        self.p2p.current_sequence()
    }
}
