use crate::domain::PeerId;
use instant::{Duration, Instant};
use std::collections::HashMap;
use uuid::Uuid;

/// Connection status of a peer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Peer is connected and responsive
    Connected,
    /// Peer disconnected, but within grace period
    Disconnected { since: Instant },
    /// Grace period expired, peer is considered gone
    TimedOut,
}

/// State tracking for a connected peer
#[derive(Debug, Clone)]
pub struct PeerState {
    /// When this peer connected
    pub connected_at: Instant,
    /// Last time we received any message from this peer
    pub last_seen: Instant,
    /// Current connection status
    pub status: ConnectionStatus,
    /// Participant ID associated with this peer (if known)
    pub participant_id: Option<Uuid>,
    /// Participant name (if known)
    pub name: Option<String>,
    /// Whether this peer is a host
    pub is_host: bool,
}

impl PeerState {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            connected_at: now,
            last_seen: now,
            status: ConnectionStatus::Connected,
            participant_id: None,
            name: None,
            is_host: false,
        }
    }

    /// Update the last seen timestamp
    pub fn update_last_seen(&mut self) {
        self.last_seen = Instant::now();
    }

    /// Set participant information
    pub fn set_participant_info(&mut self, participant_id: Uuid, name: String, is_host: bool) {
        self.participant_id = Some(participant_id);
        self.name = Some(name);
        self.is_host = is_host;
    }

    /// Check if we know this peer's participant ID
    pub fn has_participant_info(&self) -> bool {
        self.participant_id.is_some()
    }

    /// Mark as disconnected
    pub fn mark_disconnected(&mut self) {
        self.status = ConnectionStatus::Disconnected {
            since: Instant::now(),
        };
    }

    /// Check if grace period has expired
    pub fn check_grace_period(&mut self, grace_period: Duration) -> bool {
        match self.status {
            ConnectionStatus::Disconnected { since } => {
                if since.elapsed() >= grace_period {
                    self.status = ConnectionStatus::TimedOut;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Check if peer is timed out
    pub fn is_timed_out(&self) -> bool {
        matches!(self.status, ConnectionStatus::TimedOut)
    }

    /// Check if peer is disconnected (but may still be in grace period)
    pub fn is_disconnected(&self) -> bool {
        matches!(
            self.status,
            ConnectionStatus::Disconnected { .. } | ConnectionStatus::TimedOut
        )
    }
}

impl Default for PeerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages state for all connected peers
#[derive(Debug, Default)]
pub struct PeerRegistry {
    peers: HashMap<PeerId, PeerState>,
    grace_period: Duration,
}

impl PeerRegistry {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
            grace_period: Duration::from_secs(30),
        }
    }

    pub fn with_grace_period(grace_period: Duration) -> Self {
        Self {
            peers: HashMap::new(),
            grace_period,
        }
    }

    /// Add a new peer
    pub fn add_peer(&mut self, peer_id: PeerId) {
        self.peers.insert(peer_id, PeerState::new());
    }

    /// Mark a peer as disconnected (starts grace period)
    pub fn mark_peer_disconnected(&mut self, peer_id: &PeerId) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.mark_disconnected();
        }
    }

    /// Remove a peer completely (after timeout)
    pub fn remove_peer(&mut self, peer_id: &PeerId) -> Option<PeerState> {
        self.peers.remove(peer_id)
    }

    /// Get peer state (mutable)
    pub fn get_peer_mut(&mut self, peer_id: &PeerId) -> Option<&mut PeerState> {
        self.peers.get_mut(peer_id)
    }

    /// Get peer state (immutable)
    pub fn get_peer(&self, peer_id: &PeerId) -> Option<&PeerState> {
        self.peers.get(peer_id)
    }

    /// Update last seen for a peer
    pub fn update_last_seen(&mut self, peer_id: &PeerId) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.update_last_seen();
        }
    }

    /// Check all disconnected peers for grace period expiration
    /// Returns list of peers that have timed out
    pub fn check_grace_periods(&mut self) -> Vec<PeerId> {
        let mut timed_out = Vec::new();

        for (peer_id, peer_state) in self.peers.iter_mut() {
            if peer_state.check_grace_period(self.grace_period) {
                timed_out.push(*peer_id);
            }
        }

        timed_out
    }

    /// Find peer ID by participant ID
    pub fn find_by_participant_id(&self, participant_id: Uuid) -> Option<PeerId> {
        self.peers
            .iter()
            .find(|(_, state)| state.participant_id == Some(participant_id))
            .map(|(peer_id, _)| *peer_id)
    }

    /// Find the host peer
    pub fn find_host(&self) -> Option<(PeerId, &PeerState)> {
        self.peers
            .iter()
            .find(|(_, state)| state.is_host && !state.is_timed_out())
            .map(|(peer_id, state)| (*peer_id, state))
    }

    /// Get all peers
    pub fn all_peers(&self) -> impl Iterator<Item = (&PeerId, &PeerState)> {
        self.peers.iter()
    }

    /// Get count of connected peers (not timed out)
    pub fn peer_count(&self) -> usize {
        self.peers
            .values()
            .filter(|state| !state.is_timed_out())
            .count()
    }

    /// Check if a peer is the host
    pub fn is_peer_host(&self, peer_id: &PeerId) -> bool {
        self.peers
            .get(peer_id)
            .map(|state| state.is_host && !state.is_timed_out())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_status() {
        let mut state = PeerState::new();
        assert_eq!(state.status, ConnectionStatus::Connected);

        state.mark_disconnected();
        assert!(state.is_disconnected());
        assert!(!state.is_timed_out());
    }

    #[test]
    fn test_grace_period_expiry() {
        let mut state = PeerState::new();
        state.mark_disconnected();

        // Should not expire immediately
        let expired = state.check_grace_period(Duration::from_secs(30));
        assert!(!expired);
        assert!(!state.is_timed_out());

        // Simulate time passage (use zero duration for testing)
        let expired = state.check_grace_period(Duration::from_millis(0));
        assert!(expired);
        assert!(state.is_timed_out());
    }

    #[test]
    fn test_peer_registry_disconnection() {
        let mut registry = PeerRegistry::new();
        let peer_id = PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()));

        registry.add_peer(peer_id);
        assert_eq!(registry.peer_count(), 1);

        registry.mark_peer_disconnected(&peer_id);
        assert_eq!(registry.peer_count(), 1); // Still counted during grace period

        // After timeout
        registry
            .get_peer_mut(&peer_id)
            .unwrap()
            .check_grace_period(Duration::from_millis(0));
        assert_eq!(registry.peer_count(), 0); // No longer counted
    }

    #[test]
    fn test_find_host_excludes_timed_out() {
        let mut registry = PeerRegistry::new();
        let host_peer = PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()));

        registry.add_peer(host_peer);
        registry
            .get_peer_mut(&host_peer)
            .unwrap()
            .set_participant_info(Uuid::new_v4(), "Host".to_string(), true);

        assert!(registry.find_host().is_some());

        // Mark as timed out
        registry.mark_peer_disconnected(&host_peer);
        registry
            .get_peer_mut(&host_peer)
            .unwrap()
            .check_grace_period(Duration::from_millis(0));

        // Should not find timed-out host
        assert!(registry.find_host().is_none());
    }
}
