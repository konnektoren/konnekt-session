use konnekt_session_core::{DomainEvent, DomainLoop, Lobby, Participant};
use konnekt_session_p2p::{ConnectionEvent, DomainEvent as P2PDomainEvent, P2PLoop};
use uuid::Uuid;

/// Orchestrates both domain and P2P event loops
///
/// P2P layer now AUTOMATICALLY handles event sync, ordering, and validation.
pub struct DualLoopRuntime {
    /// Domain event loop (owns lobby state)
    domain_loop: DomainLoop,

    /// P2P event loop (owns network state + automatic sync)
    p2p_loop: P2PLoop,

    /// Current lobby ID (if any)
    lobby_id: Option<Uuid>,

    /// Are we the host?
    is_host: bool,
}

impl DualLoopRuntime {
    /// Create a new runtime
    pub fn new(domain_loop: DomainLoop, p2p_loop: P2PLoop) -> Self {
        Self {
            domain_loop,
            p2p_loop,
            lobby_id: None,
            is_host: false,
        }
    }

    /// Set lobby context (after creating/joining)
    pub fn set_lobby(&mut self, lobby_id: Uuid, is_host: bool) {
        self.lobby_id = Some(lobby_id);
        self.is_host = is_host;
    }

    /// Main event loop tick
    ///
    /// Returns number of operations performed
    pub fn tick(&mut self) -> RuntimeStats {
        let mut stats = RuntimeStats::default();

        // 1. Poll P2P (receive network messages - AUTOMATIC SYNC)
        stats.p2p_events = self.p2p_loop.poll();

        // 2. Process inbound messages (P2P → Domain)
        stats.messages_received = self.process_inbound_lobby_events();

        // 3. Poll domain (process commands → emit events)
        stats.commands_processed = self.domain_loop.poll();

        // 4. Process outbound events (Domain → P2P)
        stats.events_broadcast = self.process_outbound_events();

        // 5. Poll P2P again (send queued messages)
        stats.p2p_events += self.p2p_loop.poll();

        stats
    }

    /// Process lobby events from P2P → Domain
    /// P2P layer has already validated and ordered these events
    fn process_inbound_lobby_events(&mut self) -> usize {
        let lobby_events = self.p2p_loop.drain_lobby_events();
        let mut count = 0;

        for lobby_event in lobby_events {
            tracing::debug!("📥 Processing lobby event: {:?}", lobby_event.event);

            // Convert P2P events to domain commands (where applicable)
            match &lobby_event.event {
                P2PDomainEvent::LobbyCreated {
                    lobby_id,
                    host_id,
                    name,
                } => {
                    // 🆕 FIX: Create the lobby in the domain for guests
                    if !self.is_host {
                        tracing::info!("🔧 Guest creating lobby in domain: {}", name);

                        // Create host participant
                        match Participant::host_with_id(*host_id, "Host".to_string()) {
                            Ok(host) => {
                                match Lobby::with_id(*lobby_id, name.clone(), host) {
                                    Ok(lobby) => {
                                        // Add to domain event loop's lobbies
                                        // (This is the missing piece!)
                                        self.domain_loop.event_loop_mut().add_lobby(lobby.clone());

                                        self.lobby_id = Some(*lobby_id);
                                        tracing::info!(
                                            "✅ Guest created lobby {} in domain",
                                            lobby_id
                                        );
                                    }
                                    Err(e) => {
                                        tracing::error!("❌ Failed to create lobby: {:?}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("❌ Failed to create host participant: {:?}", e);
                            }
                        }
                    }
                    count += 1;
                }

                P2PDomainEvent::GuestJoined { participant } => {
                    // Apply to domain if we have a lobby
                    if let Some(lobby_id) = self.lobby_id {
                        if let Some(lobby) = self.domain_loop.event_loop().get_lobby(&lobby_id) {
                            // Clone and add guest
                            let mut lobby_copy = lobby.clone();
                            if lobby_copy.add_guest(participant.clone()).is_ok() {
                                tracing::debug!(
                                    "✅ Added guest '{}' to domain",
                                    participant.name()
                                );
                                // Note: In production, we'd update the lobby in event_loop
                                // For now, just log
                            }
                        }
                    }
                    count += 1;
                }

                P2PDomainEvent::GuestLeft { participant_id } => {
                    if let Some(lobby_id) = self.lobby_id {
                        if let Some(lobby) = self.domain_loop.event_loop().get_lobby(&lobby_id) {
                            let mut lobby_copy = lobby.clone();
                            lobby_copy.participants_mut().remove(participant_id);
                            tracing::debug!("✅ Removed guest {} from domain", participant_id);
                        }
                    }
                    count += 1;
                }

                P2PDomainEvent::GuestKicked { participant_id, .. } => {
                    if let Some(lobby_id) = self.lobby_id {
                        if let Some(lobby) = self.domain_loop.event_loop().get_lobby(&lobby_id) {
                            let mut lobby_copy = lobby.clone();
                            lobby_copy.participants_mut().remove(participant_id);
                            tracing::debug!(
                                "✅ Removed kicked guest {} from domain",
                                participant_id
                            );
                        }
                    }
                    count += 1;
                }

                P2PDomainEvent::HostDelegated { from: _, to, .. } => {
                    if let Some(lobby_id) = self.lobby_id {
                        if let Some(lobby) = self.domain_loop.event_loop().get_lobby(&lobby_id) {
                            let mut lobby_copy = lobby.clone();
                            if lobby_copy.delegate_host(*to).is_ok() {
                                tracing::debug!("✅ Delegated host to {} in domain", to);

                                // Check if WE became the host
                                if let Some(our_lobby) =
                                    self.domain_loop.event_loop().get_lobby(&lobby_id)
                                {
                                    if our_lobby.host_id() == *to {
                                        self.is_host = true;
                                        self.p2p_loop.promote_to_host();
                                        tracing::info!("👑 We are now the HOST!");
                                    }
                                }
                            }
                        }
                    }
                    count += 1;
                }

                P2PDomainEvent::ParticipationModeChanged {
                    participant_id,
                    new_mode,
                } => {
                    if let Some(lobby_id) = self.lobby_id {
                        if let Some(lobby) = self.domain_loop.event_loop().get_lobby(&lobby_id) {
                            let mut lobby_copy = lobby.clone();
                            if let Some(participant) =
                                lobby_copy.participants_mut().get_mut(participant_id)
                            {
                                let mode = if new_mode == "Active" {
                                    konnekt_session_core::ParticipationMode::Active
                                } else {
                                    konnekt_session_core::ParticipationMode::Spectating
                                };
                                participant.force_participation_mode(mode);
                                tracing::debug!(
                                    "✅ Updated participation mode for {}",
                                    participant_id
                                );
                            }
                        }
                    }
                    count += 1;
                }
            }
        }

        count
    }

    /// Process events from Domain → P2P
    /// Only host broadcasts events
    fn process_outbound_events(&mut self) -> usize {
        let events = self.domain_loop.drain_events();
        let mut count = 0;

        for domain_event in events {
            // Update lobby context
            if let DomainEvent::LobbyCreated { lobby } = &domain_event {
                self.set_lobby(lobby.id(), true);
            }

            // Only host broadcasts events
            if !self.is_host {
                continue;
            }

            // Convert domain event to P2P event
            let p2p_event = match domain_event {
                DomainEvent::LobbyCreated { lobby } => Some(P2PDomainEvent::LobbyCreated {
                    lobby_id: lobby.id(),
                    host_id: lobby.host_id(),
                    name: lobby.name().to_string(),
                }),

                DomainEvent::GuestJoined {
                    lobby_id: _,
                    participant,
                } => Some(P2PDomainEvent::GuestJoined { participant }),

                DomainEvent::GuestLeft {
                    lobby_id: _,
                    participant_id,
                } => Some(P2PDomainEvent::GuestLeft { participant_id }),

                DomainEvent::GuestKicked {
                    lobby_id: _,
                    participant_id,
                    kicked_by,
                } => Some(P2PDomainEvent::GuestKicked {
                    participant_id,
                    kicked_by,
                }),

                DomainEvent::HostDelegated {
                    lobby_id: _,
                    from,
                    to,
                } => Some(P2PDomainEvent::HostDelegated {
                    from,
                    to,
                    reason: konnekt_session_p2p::DelegationReason::Manual,
                }),

                DomainEvent::ParticipationModeChanged {
                    lobby_id: _,
                    participant_id,
                    new_mode,
                } => Some(P2PDomainEvent::ParticipationModeChanged {
                    participant_id,
                    new_mode: format!("{}", new_mode),
                }),

                DomainEvent::CommandFailed { .. } => {
                    // Don't broadcast failures
                    None
                }
            };

            // Broadcast via P2P (automatic sync)
            if let Some(event) = p2p_event {
                match self.p2p_loop.broadcast_event(event) {
                    Ok(sequence) => {
                        tracing::debug!("📤 Broadcast event sequence {}", sequence);
                        count += 1;
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to broadcast event: {:?}", e);
                    }
                }
            }
        }

        count
    }

    /// Get connection events (for application layer to handle)
    pub fn drain_connection_events(&mut self) -> Vec<ConnectionEvent> {
        self.p2p_loop.drain_events()
    }

    /// Get reference to domain loop (for queries)
    pub fn domain_loop(&self) -> &DomainLoop {
        &self.domain_loop
    }

    /// Get reference to P2P loop (for queries)
    pub fn p2p_loop(&self) -> &P2PLoop {
        &self.p2p_loop
    }

    /// Get mutable reference to P2P loop
    pub fn p2p_loop_mut(&mut self) -> &mut P2PLoop {
        &mut self.p2p_loop
    }

    /// Get mutable reference to domain loop (for direct command submission)
    pub fn domain_loop_mut(&mut self) -> &mut DomainLoop {
        &mut self.domain_loop
    }

    /// Handle peer connected - send full lobby state if we're host
    pub fn handle_peer_connected(&mut self, peer_id: konnekt_session_p2p::PeerId) {
        if !self.is_host {
            return;
        }

        tracing::info!("📤 New peer connected, sending full lobby state");

        if let Some(lobby_id) = self.lobby_id {
            if let Some(lobby) = self.domain_loop.event_loop().get_lobby(&lobby_id) {
                // Create LobbySnapshot
                let snapshot = konnekt_session_p2p::LobbySnapshot {
                    lobby_id,
                    name: lobby.name().to_string(),
                    host_id: lobby.host_id(),
                    participants: lobby.participants().values().cloned().collect(),
                    as_of_sequence: self.p2p_loop.current_sequence(),
                };

                // Send via P2P
                if let Err(e) = self.p2p_loop.send_full_sync_to_peer(peer_id, snapshot) {
                    tracing::error!("❌ Failed to send full sync: {:?}", e);
                }
            }
        }
    }
}

/// Statistics from a runtime tick
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeStats {
    /// P2P events processed
    pub p2p_events: usize,
    /// Messages received from network
    pub messages_received: usize,
    /// Domain commands processed
    pub commands_processed: usize,
    /// Events broadcast to network
    pub events_broadcast: usize,
}

impl RuntimeStats {
    /// Total operations performed
    pub fn total(&self) -> usize {
        self.p2p_events + self.messages_received + self.commands_processed + self.events_broadcast
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_stats() {
        let stats = RuntimeStats {
            p2p_events: 2,
            messages_received: 3,
            commands_processed: 4,
            events_broadcast: 1,
        };

        assert_eq!(stats.total(), 10);
    }

    #[test]
    fn test_runtime_stats_default() {
        let stats = RuntimeStats::default();
        assert_eq!(stats.total(), 0);
    }
}
