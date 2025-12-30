use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::PeerId;

/// Enforces 1:1 bidirectional mapping between peers and participants
///
/// This is a core domain invariant: every peer corresponds to exactly one participant,
/// and every participant corresponds to exactly one peer.
#[derive(Debug, Default, Clone)]
pub struct PeerParticipantMap {
    /// Peer ID → Participant ID
    peer_to_participant: HashMap<PeerId, Uuid>,
    /// Participant ID → Peer ID
    participant_to_peer: HashMap<Uuid, PeerId>,
}

impl PeerParticipantMap {
    /// Create a new empty mapping
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a peer-participant mapping (enforces 1:1)
    ///
    /// If either the peer or participant was already mapped to something else,
    /// those old mappings are removed first.
    pub fn register(&mut self, peer_id: PeerId, participant_id: Uuid) {
        // Remove any existing mappings for this peer
        if let Some(old_participant) = self.peer_to_participant.get(&peer_id) {
            self.participant_to_peer.remove(old_participant);
        }

        // Remove any existing mappings for this participant
        if let Some(old_peer) = self.participant_to_peer.get(&participant_id) {
            self.peer_to_participant.remove(old_peer);
        }

        // Create new bidirectional mapping
        self.peer_to_participant.insert(peer_id, participant_id);
        self.participant_to_peer.insert(participant_id, peer_id);
    }

    /// Remove mapping for a peer, returning the participant ID if it existed
    pub fn remove_by_peer(&mut self, peer_id: &PeerId) -> Option<Uuid> {
        if let Some(participant_id) = self.peer_to_participant.remove(peer_id) {
            self.participant_to_peer.remove(&participant_id);
            Some(participant_id)
        } else {
            None
        }
    }

    /// Remove mapping for a participant, returning the peer ID if it existed
    pub fn remove_by_participant(&mut self, participant_id: &Uuid) -> Option<PeerId> {
        if let Some(peer_id) = self.participant_to_peer.remove(participant_id) {
            self.peer_to_participant.remove(&peer_id);
            Some(peer_id)
        } else {
            None
        }
    }

    /// Get participant ID for a peer
    pub fn get_participant(&self, peer_id: &PeerId) -> Option<Uuid> {
        self.peer_to_participant.get(peer_id).copied()
    }

    /// Get peer ID for a participant
    pub fn get_peer(&self, participant_id: &Uuid) -> Option<PeerId> {
        self.participant_to_peer.get(participant_id).copied()
    }

    /// Check if a peer is registered
    pub fn contains_peer(&self, peer_id: &PeerId) -> bool {
        self.peer_to_participant.contains_key(peer_id)
    }

    /// Check if a participant is registered
    pub fn contains_participant(&self, participant_id: &Uuid) -> bool {
        self.participant_to_peer.contains_key(participant_id)
    }

    /// Get all peer IDs
    pub fn all_peers(&self) -> impl Iterator<Item = &PeerId> {
        self.peer_to_participant.keys()
    }

    /// Get all participant IDs
    pub fn all_participants(&self) -> impl Iterator<Item = &Uuid> {
        self.participant_to_peer.keys()
    }

    /// Get the number of mappings
    pub fn len(&self) -> usize {
        debug_assert_eq!(
            self.peer_to_participant.len(),
            self.participant_to_peer.len(),
            "Bidirectional map invariant violated"
        );
        self.peer_to_participant.len()
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.peer_to_participant.is_empty()
    }

    /// Clear all mappings
    pub fn clear(&mut self) {
        self.peer_to_participant.clear();
        self.participant_to_peer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_peer() -> PeerId {
        PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()))
    }

    #[test]
    fn test_register_mapping() {
        let mut map = PeerParticipantMap::new();
        let peer = create_peer();
        let participant = Uuid::new_v4();

        map.register(peer, participant);

        assert_eq!(map.get_participant(&peer), Some(participant));
        assert_eq!(map.get_peer(&participant), Some(peer));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_remove_by_peer() {
        let mut map = PeerParticipantMap::new();
        let peer = create_peer();
        let participant = Uuid::new_v4();

        map.register(peer, participant);
        let removed = map.remove_by_peer(&peer);

        assert_eq!(removed, Some(participant));
        assert_eq!(map.get_participant(&peer), None);
        assert_eq!(map.get_peer(&participant), None);
        assert!(map.is_empty());
    }

    #[test]
    fn test_remove_by_participant() {
        let mut map = PeerParticipantMap::new();
        let peer = create_peer();
        let participant = Uuid::new_v4();

        map.register(peer, participant);
        let removed = map.remove_by_participant(&participant);

        assert_eq!(removed, Some(peer));
        assert_eq!(map.get_participant(&peer), None);
        assert_eq!(map.get_peer(&participant), None);
        assert!(map.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_peer() {
        let mut map = PeerParticipantMap::new();
        let peer = create_peer();

        let removed = map.remove_by_peer(&peer);
        assert_eq!(removed, None);
    }

    #[test]
    fn test_remove_nonexistent_participant() {
        let mut map = PeerParticipantMap::new();
        let participant = Uuid::new_v4();

        let removed = map.remove_by_participant(&participant);
        assert_eq!(removed, None);
    }

    #[test]
    fn test_one_to_one_enforcement_peer_reuse() {
        let mut map = PeerParticipantMap::new();
        let peer = create_peer();
        let participant1 = Uuid::new_v4();
        let participant2 = Uuid::new_v4();

        // Register peer -> participant1
        map.register(peer, participant1);
        assert_eq!(map.len(), 1);

        // Register same peer -> participant2 (should remove participant1 mapping)
        map.register(peer, participant2);

        assert_eq!(map.get_participant(&peer), Some(participant2));
        assert_eq!(map.get_peer(&participant1), None);
        assert_eq!(map.get_peer(&participant2), Some(peer));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_one_to_one_enforcement_participant_reuse() {
        let mut map = PeerParticipantMap::new();
        let peer1 = create_peer();
        let peer2 = create_peer();
        let participant = Uuid::new_v4();

        // Register peer1 -> participant
        map.register(peer1, participant);
        assert_eq!(map.len(), 1);

        // Register peer2 -> same participant (should remove peer1 mapping)
        map.register(peer2, participant);

        assert_eq!(map.get_participant(&peer1), None);
        assert_eq!(map.get_participant(&peer2), Some(participant));
        assert_eq!(map.get_peer(&participant), Some(peer2));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_multiple_independent_mappings() {
        let mut map = PeerParticipantMap::new();
        let peer1 = create_peer();
        let peer2 = create_peer();
        let participant1 = Uuid::new_v4();
        let participant2 = Uuid::new_v4();

        map.register(peer1, participant1);
        map.register(peer2, participant2);

        assert_eq!(map.get_participant(&peer1), Some(participant1));
        assert_eq!(map.get_participant(&peer2), Some(participant2));
        assert_eq!(map.get_peer(&participant1), Some(peer1));
        assert_eq!(map.get_peer(&participant2), Some(peer2));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_contains_peer() {
        let mut map = PeerParticipantMap::new();
        let peer = create_peer();
        let participant = Uuid::new_v4();

        assert!(!map.contains_peer(&peer));

        map.register(peer, participant);
        assert!(map.contains_peer(&peer));

        map.remove_by_peer(&peer);
        assert!(!map.contains_peer(&peer));
    }

    #[test]
    fn test_contains_participant() {
        let mut map = PeerParticipantMap::new();
        let peer = create_peer();
        let participant = Uuid::new_v4();

        assert!(!map.contains_participant(&participant));

        map.register(peer, participant);
        assert!(map.contains_participant(&participant));

        map.remove_by_participant(&participant);
        assert!(!map.contains_participant(&participant));
    }

    #[test]
    fn test_all_peers() {
        let mut map = PeerParticipantMap::new();
        let peer1 = create_peer();
        let peer2 = create_peer();

        map.register(peer1, Uuid::new_v4());
        map.register(peer2, Uuid::new_v4());

        let peers: Vec<_> = map.all_peers().copied().collect();
        assert_eq!(peers.len(), 2);
        assert!(peers.contains(&peer1));
        assert!(peers.contains(&peer2));
    }

    #[test]
    fn test_all_participants() {
        let mut map = PeerParticipantMap::new();
        let participant1 = Uuid::new_v4();
        let participant2 = Uuid::new_v4();

        map.register(create_peer(), participant1);
        map.register(create_peer(), participant2);

        let participants: Vec<_> = map.all_participants().copied().collect();
        assert_eq!(participants.len(), 2);
        assert!(participants.contains(&participant1));
        assert!(participants.contains(&participant2));
    }

    #[test]
    fn test_clear() {
        let mut map = PeerParticipantMap::new();
        map.register(create_peer(), Uuid::new_v4());
        map.register(create_peer(), Uuid::new_v4());

        assert_eq!(map.len(), 2);

        map.clear();

        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }

    #[test]
    fn test_bidirectional_invariant() {
        let mut map = PeerParticipantMap::new();

        // Add several mappings
        for _ in 0..10 {
            map.register(create_peer(), Uuid::new_v4());
        }

        // Verify bidirectional invariant
        for peer in map.all_peers() {
            let participant = map.get_participant(peer).unwrap();
            assert_eq!(map.get_peer(&participant), Some(*peer));
        }

        for participant in map.all_participants() {
            let peer = map.get_peer(participant).unwrap();
            assert_eq!(map.get_participant(&peer), Some(*participant));
        }
    }
}
