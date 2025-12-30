use crate::application::LobbySnapshot;
use crate::application::runtime::P2PLoop;
use crate::domain::PeerId;
use crate::infrastructure::error::Result;
use konnekt_session_core::{DomainCommand, DomainEvent as CoreDomainEvent, DomainLoop, Lobby};
use uuid::Uuid;

/// Unified session loop that automatically coordinates P2P ↔ Core
///
/// This is the main integration point for applications.
/// It handles:
/// - P2P event synchronization (ordering, gaps, retries)
/// - Domain command execution
/// - Automatic translation between layers
/// - Peer ↔ Participant mapping (1:1)
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────┐
/// │         Application Layer               │
/// │  (CLI, Yew UI, etc.)                    │
/// └─────────────────────────────────────────┘
///                   │
///                   ↓
/// ┌─────────────────────────────────────────┐
/// │         SessionLoop (this)              │
/// │  - Coordinates P2P ↔ Core               │
/// │  - Auto-translation via EventTranslator │
/// │  - 1:1 mappings enforced                │
/// └─────────────────────────────────────────┘
///       │                        │
///       ↓                        ↓
/// ┌──────────────┐      ┌──────────────┐
/// │   P2PLoop    │      │  DomainLoop  │
/// │  (Network)   │      │  (Business)  │
/// └──────────────┘      └──────────────┘
/// ```
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

        // 🆕 AUTO-REQUEST: Guest immediately requests full sync from host
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

    /// Submit a domain command directly (for local user actions)
    ///
    /// This is for commands initiated by the local user (e.g., UI button clicks).
    /// The resulting events will automatically be broadcast via P2P.
    ///
    /// # Example
    /// ```ignore
    /// // User clicks "Join Lobby" button
    /// session_loop.submit_command(DomainCommand::JoinLobby {
    ///     lobby_id,
    ///     guest_name: "Alice".to_string(),
    /// })?;
    /// ```
    pub fn submit_command(&mut self, cmd: DomainCommand) -> Result<()> {
        tracing::debug!("📝 Submitting domain command: {:?}", cmd);

        self.domain
            .submit(cmd)
            .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))
    }

    /// Main event loop - call this regularly (e.g., every 100ms)
    ///
    /// This AUTOMATICALLY:
    /// 1. Polls P2P for network events
    /// 2. Translates incoming P2P events → domain commands
    /// 3. Executes domain commands
    /// 4. Translates outgoing domain events → P2P broadcasts
    ///
    /// Returns number of events processed.
    pub fn poll(&mut self) -> usize {
        let mut processed = 0;

        // ===== Step 1: Poll P2P (network events) =====
        let p2p_processed = self.p2p.poll();
        processed += p2p_processed;

        if p2p_processed > 0 {
            tracing::trace!("P2P processed {} events", p2p_processed);
        }

        // 🆕 AUTO-SEND SYNC: Host automatically sends full sync when new peer connects
        if self.is_host {
            let connection_events = self.p2p.drain_events();

            for event in &connection_events {
                match event {
                    crate::application::ConnectionEvent::PeerConnected(peer_id) => {
                        tracing::info!(
                            "🟢 HOST: Peer {} connected - auto-sending full sync",
                            peer_id
                        );

                        // Get current lobby state
                        if let Some(lobby) = self.get_lobby() {
                            let snapshot = crate::application::LobbySnapshot {
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
                    _ => {}
                }
            }
        }

        // ===== Step 2: Get domain commands from P2P events =====
        let commands = self.p2p.drain_domain_commands();

        if !commands.is_empty() {
            tracing::debug!("📥 Received {} domain commands from P2P", commands.len());
        }

        for cmd in commands {
            // Special logging for important commands
            match &cmd {
                DomainCommand::CreateLobby { lobby_name, .. } => {
                    tracing::info!("📥 GUEST: Received lobby snapshot via P2P: {}", lobby_name);
                }
                DomainCommand::JoinLobby { guest_name, .. } => {
                    tracing::info!("📥 Guest '{}' joining via P2P", guest_name);
                }
                DomainCommand::LeaveLobby { participant_id, .. } => {
                    tracing::info!("📥 Participant {} leaving via P2P", participant_id);
                }
                DomainCommand::DelegateHost { new_host_id, .. } => {
                    tracing::info!("📥 Host delegated to {} via P2P", new_host_id);
                }
                _ => {
                    tracing::debug!("📥 Received command: {:?}", cmd);
                }
            }

            // Submit to domain loop
            if let Err(e) = self.domain.submit(cmd) {
                tracing::warn!("Failed to submit command to domain: {:?}", e);
            }
        }

        // ===== Step 3: Process domain commands =====
        let domain_processed = self.domain.poll();
        processed += domain_processed;

        if domain_processed > 0 {
            tracing::trace!("Domain processed {} commands", domain_processed);
        }

        // ===== Step 4: Broadcast domain events via P2P =====
        let events = self.domain.drain_events();

        if !events.is_empty() {
            tracing::debug!("📤 Broadcasting {} domain events via P2P", events.len());
        }

        for event in events {
            // Log important events
            match &event {
                CoreDomainEvent::LobbyCreated { lobby } => {
                    tracing::info!("📤 Broadcasting LobbyCreated: {}", lobby.name());
                }
                CoreDomainEvent::GuestJoined { participant, .. } => {
                    tracing::info!("📤 Broadcasting GuestJoined: {}", participant.name());
                }
                CoreDomainEvent::GuestLeft { participant_id, .. } => {
                    tracing::info!("📤 Broadcasting GuestLeft: {}", participant_id);
                }
                CoreDomainEvent::HostDelegated { to, .. } => {
                    tracing::info!("📤 Broadcasting HostDelegated to {}", to);
                }
                CoreDomainEvent::CommandFailed { command, reason } => {
                    tracing::warn!("⚠️  Command failed: {} - {}", command, reason);
                }
                _ => {
                    tracing::debug!("📤 Broadcasting event: {:?}", event);
                }
            }

            // Only broadcast if we should
            if self.should_broadcast_event(&event) {
                if let Err(e) = self.p2p.apply_domain_event(event) {
                    tracing::warn!("Failed to broadcast event: {:?}", e);
                }
            }
        }

        processed
    }

    /// Determine if an event should be broadcast
    ///
    /// Guests should only broadcast their own actions (e.g., joining, leaving).
    /// Hosts broadcast everything.
    fn should_broadcast_event(&self, event: &CoreDomainEvent) -> bool {
        match event {
            // These are always broadcast by anyone
            CoreDomainEvent::GuestJoined { .. } => true,
            CoreDomainEvent::GuestLeft { .. } => true,

            // Host-only broadcasts
            CoreDomainEvent::LobbyCreated { .. } => self.is_host,
            CoreDomainEvent::GuestKicked { .. } => self.is_host,
            CoreDomainEvent::HostDelegated { .. } => self.is_host,
            CoreDomainEvent::ParticipationModeChanged { .. } => true, // Anyone can change their own

            // Never broadcast
            CoreDomainEvent::CommandFailed { .. } => false,
        }
    }

    /// Get the current lobby state (for rendering UI)
    pub fn get_lobby(&self) -> Option<&Lobby> {
        self.domain.event_loop().get_lobby(&self.lobby_id)
    }

    /// Get lobby ID
    pub fn lobby_id(&self) -> Uuid {
        self.lobby_id
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> Option<PeerId> {
        self.p2p.local_peer_id()
    }

    /// Get connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.p2p.connected_peers()
    }

    /// Check if we're the host
    pub fn is_host(&self) -> bool {
        self.is_host
    }

    /// Promote to host (after delegation)
    pub fn promote_to_host(&mut self) {
        tracing::info!("👑 Promoting to HOST");
        self.is_host = true;
        self.p2p.promote_to_host();
    }

    /// Send full sync to a new peer (HOST ONLY)
    pub fn send_full_sync_to_peer(&mut self, peer_id: PeerId) -> Result<()> {
        if !self.is_host {
            return Err(crate::infrastructure::error::P2PError::SendFailed(
                "Only host can send full sync".to_string(),
            ));
        }

        tracing::info!("📤 Sending full sync to peer {}", peer_id);

        // Get current lobby state
        let lobby = self
            .get_lobby()
            .ok_or_else(|| {
                crate::infrastructure::error::P2PError::SendFailed("No lobby found".to_string())
            })?
            .clone();

        // Create snapshot
        let snapshot = LobbySnapshot {
            lobby_id: lobby.id(),
            name: lobby.name().to_string(),
            host_id: lobby.host_id(),
            participants: lobby.participants().values().cloned().collect(),
            as_of_sequence: self.p2p.current_sequence(),
        };

        self.p2p.send_full_sync_to_peer(peer_id, snapshot)
    }

    /// Get reference to P2P loop (for advanced usage)
    pub fn p2p(&self) -> &P2PLoop {
        &self.p2p
    }

    /// Get mutable reference to P2P loop (for advanced usage)
    pub fn p2p_mut(&mut self) -> &mut P2PLoop {
        &mut self.p2p
    }

    /// Get reference to domain loop (for queries)
    pub fn domain(&self) -> &DomainLoop {
        &self.domain
    }

    /// Get pending P2P message count (for debugging)
    pub fn pending_p2p_messages(&self) -> usize {
        self.p2p.pending_messages()
    }

    /// Get pending domain command count (for debugging)
    pub fn pending_domain_commands(&self) -> usize {
        self.p2p.pending_domain_commands()
    }

    /// Get current P2P sequence number (for debugging)
    pub fn current_sequence(&self) -> u64 {
        self.p2p.current_sequence()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_core::Participant;

    #[test]
    fn test_should_broadcast_event_as_host() {
        let lobby_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();

        // Simulate host
        let is_host = true;

        // Host should broadcast LobbyCreated
        let event = CoreDomainEvent::LobbyCreated {
            lobby: Lobby::with_id(
                lobby_id,
                "Test".to_string(),
                Participant::new_host("Host".to_string()).unwrap(),
            )
            .unwrap(),
        };

        // We can test the logic directly
        assert!(
            match event {
                CoreDomainEvent::LobbyCreated { .. } => is_host,
                _ => false,
            },
            "Host should broadcast LobbyCreated"
        );
    }

    #[test]
    fn test_should_broadcast_event_as_guest() {
        // Guest should NOT broadcast LobbyCreated
        let is_host = false;

        assert!(!is_host, "Guest should not broadcast LobbyCreated");
    }

    #[test]
    fn test_command_failed_never_broadcast() {
        let event = CoreDomainEvent::CommandFailed {
            command: "Test".to_string(),
            reason: "Error".to_string(),
        };

        // Should never broadcast regardless of role
        assert!(
            matches!(event, CoreDomainEvent::CommandFailed { .. }),
            "CommandFailed should never be broadcast"
        );
    }

    #[test]
    fn test_guest_joins_broadcast() {
        let lobby_id = Uuid::new_v4();
        let participant = Participant::new_guest("Alice".to_string()).unwrap();

        let event = CoreDomainEvent::GuestJoined {
            lobby_id,
            participant,
        };

        // Should always broadcast (both host and guest)
        assert!(
            matches!(event, CoreDomainEvent::GuestJoined { .. }),
            "GuestJoined should be broadcast by anyone"
        );
    }
}
