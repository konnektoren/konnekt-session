use crate::infrastructure::error::Result;
use crate::infrastructure::transport::{P2PTransport, TransportEvent};
use konnekt_session_core::{DomainCommand, DomainEvent as CoreDomainEvent, DomainLoop, Lobby};
use uuid::Uuid;

/// Unified session loop (translation layer between domain and transport)
pub struct SessionLoopV2 {
    /// Domain layer (business logic)
    domain: DomainLoop,

    /// Transport layer (networking)
    transport: P2PTransport,

    /// Are we the host?
    is_host: bool,

    /// Lobby ID
    lobby_id: Uuid,
}

impl SessionLoopV2 {
    /// Create new session loop
    pub fn new(domain: DomainLoop, transport: P2PTransport, is_host: bool, lobby_id: Uuid) -> Self {
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

        // 1. Handle transport events (connections, snapshot requests)
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
        for payload in self.transport.poll() {
            processed += 1;

            // Deserialize domain command
            if let Ok(cmd) = serde_json::from_value::<DomainCommand>(payload) {
                // Execute in domain
                let _ = self.domain.submit(cmd);
            }
        }

        // 3. Process domain commands
        processed += self.domain.poll();

        // 4. Broadcast domain events (HOST ONLY)
        if self.is_host {
            for event in self.domain.drain_events() {
                // Translate domain event → command for guests
                if let Some(cmd) = self.event_to_command(event) {
                    if let Ok(payload) = serde_json::to_value(&cmd) {
                        let _ = self.transport.send(payload);
                    }
                }
            }
        } else {
            // Guests still need to drain events (but don't broadcast)
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
            CoreDomainEvent::ActivityStarted { activity_id, .. } => {
                Some(DomainCommand::StartActivity {
                    lobby_id: self.lobby_id,
                    activity_id,
                })
            }
            CoreDomainEvent::ActivityPlanned { metadata, .. } => {
                Some(DomainCommand::PlanActivity {
                    lobby_id: self.lobby_id,
                    metadata,
                })
            }
            CoreDomainEvent::ResultSubmitted { result, .. } => Some(DomainCommand::SubmitResult {
                lobby_id: self.lobby_id,
                result,
            }),
            CoreDomainEvent::ActivityCancelled { activity_id, .. } => {
                Some(DomainCommand::CancelActivity {
                    lobby_id: self.lobby_id,
                    activity_id,
                })
            }
            // Don't broadcast these
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
}

/// Snapshot of lobby state (for sync)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LobbySnapshot {
    lobby_id: Uuid,
    name: String,
    host_id: Uuid,
    participants: Vec<konnekt_session_core::Participant>,
}
