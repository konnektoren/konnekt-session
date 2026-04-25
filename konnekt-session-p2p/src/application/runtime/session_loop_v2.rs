use crate::infrastructure::error::Result;
use crate::infrastructure::transport::{NetworkConnection, P2PTransport, TransportEvent};
use konnekt_session_core::{DomainCommand, DomainEvent as CoreDomainEvent, DomainLoop, Lobby};
use uuid::Uuid;

/// Unified session loop (translation layer between domain and transport)
/// Generic over connection type to allow mocking in tests
pub struct SessionLoopV2<C: NetworkConnection> {
    /// Domain layer (business logic)
    domain: DomainLoop,

    /// Transport layer (networking)
    transport: P2PTransport<C>,

    /// Are we the host?
    is_host: bool,

    /// Lobby ID
    lobby_id: Uuid,
}

impl<C: NetworkConnection> SessionLoopV2<C> {
    /// Create new session loop
    pub fn new(
        domain: DomainLoop,
        transport: P2PTransport<C>,
        is_host: bool,
        lobby_id: Uuid,
    ) -> Self {
        Self {
            domain,
            transport,
            is_host,
            lobby_id,
        }
    }

    /// Submit a domain command
    pub fn submit_command(&mut self, cmd: DomainCommand) -> Result<()> {
        if self.is_host {
            // Host: execute locally
            self.domain
                .submit(cmd)
                .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))?;
        } else {
            // Guest: send to host
            let payload = serde_json::to_value(&cmd)
                .map_err(crate::infrastructure::error::P2PError::Serialization)?;
            self.transport.send_to_host(payload)?;
        }
        Ok(())
    }

    /// Main event loop
    pub fn poll(&mut self) -> usize {
        let mut processed = 0;

        // 1. Handle transport events
        for event in self.transport.drain_events() {
            match event {
                TransportEvent::PeerConnected(peer_id) => {
                    if self.is_host {
                        tracing::info!("🟢 HOST: Peer {} connected - sending snapshot", peer_id);
                        self.send_snapshot_to_peer(peer_id);
                    } else {
                        tracing::info!("🟢 GUEST: Connected to host - requesting snapshot");
                        let _ = self.transport.request_snapshot();
                    }
                }
                TransportEvent::SnapshotRequested { from } => {
                    if self.is_host {
                        tracing::info!("📥 HOST: Snapshot requested by {}", from);
                        self.send_snapshot_to_peer(from);
                    }
                }
                TransportEvent::SnapshotReceived {
                    snapshot,
                    as_of_sequence,
                } => {
                    tracing::info!("📥 GUEST: Received snapshot (seq: {})", as_of_sequence);
                    self.apply_snapshot(snapshot);
                }
            }
            processed += 1;
        }

        // 2. Poll transport for messages
        let messages = self.transport.poll();

        if !messages.is_empty() {
            tracing::debug!("📥 Received {} messages from transport", messages.len());
        }

        for payload in messages {
            processed += 1;

            if let Ok(cmd) = serde_json::from_value::<DomainCommand>(payload.clone()) {
                tracing::debug!("📥 Processing command: {:?}", std::mem::discriminant(&cmd));

                // Log details for important commands
                match &cmd {
                    DomainCommand::JoinLobby { guest_name, .. } => {
                        tracing::info!("👤 Guest '{}' wants to join", guest_name);
                    }
                    DomainCommand::SubmitResult { result, run_id, .. } => {
                        tracing::info!(
                            "📊 Result from participant {} for run {}",
                            result.participant_id,
                            run_id
                        );
                    }
                    _ => {}
                }

                // ✅ FIX: Execute in domain FIRST
                if let Err(e) = self.domain.submit(cmd.clone()) {
                    tracing::warn!("❌ Failed to submit command to domain: {:?}", e);
                    continue; // Skip broadcast if command failed
                }

                // ✅ FIX: If host, ALWAYS broadcast to all guests (even if we executed it)
                if self.is_host {
                    tracing::debug!(
                        "📡 HOST: Broadcasting command to all peers: {:?}",
                        std::mem::discriminant(&cmd)
                    );

                    if let Ok(payload) = serde_json::to_value(&cmd) {
                        if let Err(e) = self.transport.send(payload) {
                            tracing::warn!("❌ Failed to broadcast: {:?}", e);
                        } else {
                            tracing::debug!("✅ Broadcast successful");
                        }
                    }
                }
            }
        }

        // 3. Process domain commands
        let domain_processed = self.domain.poll();
        processed += domain_processed;

        if domain_processed > 0 {
            tracing::debug!("🔧 Domain processed {} commands", domain_processed);
        }

        // 4. Broadcast HOST-INITIATED events (not guest commands)
        if self.is_host {
            for event in self.domain.drain_events() {
                tracing::debug!(
                    "📤 HOST: Processing domain event: {:?}",
                    std::mem::discriminant(&event)
                );

                match &event {
                    // ✅ Skip events that came from guest commands (already broadcast in step 2)
                    CoreDomainEvent::GuestJoined { .. } => {
                        tracing::debug!("   ↳ Skipping GuestJoined (already broadcast)");
                        continue;
                    }
                    CoreDomainEvent::ResultSubmitted { .. } => {
                        tracing::debug!("   ↳ Skipping ResultSubmitted (already broadcast)");
                        continue;
                    }
                    CoreDomainEvent::GuestLeft { .. } => {
                        tracing::debug!("   ↳ Skipping GuestLeft (already broadcast)");
                        continue;
                    }
                    CoreDomainEvent::RunEnded { .. } => {
                        tracing::debug!(
                            "   ↳ Skipping RunEnded (auto-completes on guests)"
                        );
                        continue;
                    }
                    _ => {}
                }

                // Translate HOST-initiated events → commands for guests
                if let Some(cmd) = self.event_to_command(event) {
                    tracing::debug!(
                        "   ↳ Broadcasting host-initiated event as command: {:?}",
                        std::mem::discriminant(&cmd)
                    );

                    if let Ok(payload) = serde_json::to_value(&cmd) {
                        let _ = self.transport.send(payload);
                    }
                }
            }
        } else {
            // Guests drain events (but don't broadcast)
            self.domain.drain_events();
        }

        processed
    }

    /// Send snapshot to a specific peer (HOST ONLY)
    fn send_snapshot_to_peer(&mut self, peer_id: crate::domain::PeerId) {
        if let Some(lobby) = self.get_lobby() {
            let snapshot = self.create_snapshot(lobby);
            if let Ok(snapshot_json) = serde_json::to_value(&snapshot) {
                let _ = self.transport.send_snapshot(peer_id, snapshot_json);
            }
        }
    }

    /// Create snapshot from current lobby state
    fn create_snapshot(&self, lobby: &Lobby) -> LobbySnapshot {
        LobbySnapshot {
            lobby_id: lobby.id(),
            name: lobby.name().to_string(),
            host_id: lobby.host_id(),
            participants: lobby.participants().values().cloned().collect(),
        }
    }

    /// Apply received snapshot (GUEST ONLY)
    fn apply_snapshot(&mut self, snapshot_json: serde_json::Value) {
        if let Ok(snapshot) = serde_json::from_value::<LobbySnapshot>(snapshot_json) {
            tracing::info!("📥 GUEST: Applying snapshot for lobby '{}'", snapshot.name);

            // Find host participant
            let host_participant = snapshot
                .participants
                .iter()
                .find(|p| p.is_host())
                .cloned()
                .expect("Snapshot must have a host");

            // Create lobby with host
            let create_cmd = DomainCommand::CreateLobbyWithHost {
                lobby_id: snapshot.lobby_id,
                lobby_name: snapshot.name,
                host: host_participant,
            };

            let _ = self.domain.submit(create_cmd);
            self.domain.poll();

            // Add other participants
            for participant in snapshot.participants.iter() {
                if !participant.is_host() {
                    let add_cmd = DomainCommand::AddParticipant {
                        lobby_id: snapshot.lobby_id,
                        participant: participant.clone(),
                    };
                    let _ = self.domain.submit(add_cmd);
                }
            }

            self.domain.poll();

            tracing::info!("✅ GUEST: Snapshot applied successfully");
        }
    }

    /// Translate domain event to command for guests
    fn event_to_command(&self, event: CoreDomainEvent) -> Option<DomainCommand> {
        match event {
            CoreDomainEvent::GuestJoined { participant, .. } => {
                Some(DomainCommand::AddParticipant {
                    lobby_id: self.lobby_id,
                    participant,
                })
            }
            CoreDomainEvent::RunStarted { run_id, config, .. } => {
                let required_submitters = self.domain.event_loop()
                    .get_run(&run_id)
                    .map(|r| r.required_submitters().iter().cloned().collect())
                    .unwrap_or_default();
                Some(DomainCommand::SyncRunStarted {
                    lobby_id: self.lobby_id,
                    run_id,
                    config,
                    required_submitters,
                })
            }
            CoreDomainEvent::ActivityQueued { config, .. } => {
                Some(DomainCommand::QueueActivity {
                    lobby_id: self.lobby_id,
                    config,
                })
            }
            CoreDomainEvent::ResultSubmitted { run_id, result, .. } => {
                Some(DomainCommand::SubmitResult {
                    lobby_id: self.lobby_id,
                    run_id,
                    result,
                })
            }
            CoreDomainEvent::RunEnded {
                run_id: _,
                results: _,
                ..
            } => {
                // Guests auto-complete when they process all SubmitResult commands
                None // Guest will auto-complete when they receive all results
            }
            _ => None,
        }
    }

    /// Get current lobby
    pub fn get_lobby(&self) -> Option<&Lobby> {
        self.domain.event_loop().get_lobby(&self.lobby_id)
    }

    pub fn lobby_id(&self) -> Uuid {
        self.lobby_id
    }

    pub fn is_host(&self) -> bool {
        self.is_host
    }

    pub fn connected_peers(&self) -> Vec<crate::domain::PeerId> {
        self.transport.connected_peers()
    }

    pub fn get_active_run(&self) -> Option<&konnekt_session_core::ActivityRun> {
        let run_id = self.get_lobby()?.active_run_id()?;
        self.domain.event_loop().get_run(&run_id)
    }
}

// Type alias for production use
pub type MatchboxSessionLoop = SessionLoopV2<crate::infrastructure::connection::MatchboxConnection>;

/// Snapshot of lobby state (for sync)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LobbySnapshot {
    lobby_id: Uuid,
    name: String,
    host_id: Uuid,
    participants: Vec<konnekt_session_core::Participant>,
}
