use crate::domain::event::LobbyEvent;
use std::collections::VecDeque;

/// Bounded event log that keeps the last N events
///
/// This is used for:
/// - Late joiner sync (send recent events to new guests)
/// - Event replay for debugging
/// - Detecting missing events
#[derive(Debug, Clone)]
pub struct EventLog {
    /// Maximum events to keep in memory
    max_size: usize,

    /// Circular buffer of events (oldest at front)
    events: VecDeque<LobbyEvent>,

    /// Next sequence number to assign (host only)
    next_sequence: u64,

    /// Highest sequence number we've seen (all peers)
    highest_seen: u64,
}

impl EventLog {
    /// Create a new event log with default capacity (100 events)
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    /// Create a new event log with custom capacity
    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            max_size,
            events: VecDeque::with_capacity(max_size),
            next_sequence: 1, // Start at 1 (0 is reserved for "unassigned")
            highest_seen: 0,
        }
    }

    /// Append an event (host only - assigns sequence number)
    pub fn append(&mut self, mut event: LobbyEvent) -> u64 {
        event.sequence = self.next_sequence;
        self.next_sequence += 1;

        self.add_event(event);
        self.next_sequence - 1 // Return the assigned sequence
    }

    /// Add an event that already has a sequence number (guests receiving from host)
    pub fn add_event(&mut self, event: LobbyEvent) {
        // Track highest sequence we've seen
        if event.sequence > self.highest_seen {
            self.highest_seen = event.sequence;
        }

        // Add to buffer
        self.events.push_back(event);

        // Evict oldest if over capacity
        if self.events.len() > self.max_size {
            self.events.pop_front();
        }
    }

    /// Get event by sequence number
    pub fn get(&self, sequence: u64) -> Option<&LobbyEvent> {
        self.events.iter().find(|e| e.sequence == sequence)
    }

    /// Get all events after a given sequence (for late joiners)
    pub fn get_since(&self, sequence: u64) -> Vec<LobbyEvent> {
        self.events
            .iter()
            .filter(|e| e.sequence > sequence)
            .cloned()
            .collect()
    }

    /// Get the last N events
    pub fn get_last(&self, n: usize) -> Vec<LobbyEvent> {
        self.events
            .iter()
            .rev()
            .take(n)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Get all events
    pub fn all_events(&self) -> Vec<LobbyEvent> {
        self.events.iter().cloned().collect()
    }

    /// Get the highest sequence number we've seen
    pub fn highest_sequence(&self) -> u64 {
        self.highest_seen
    }

    /// Get the next sequence number to assign (host only)
    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    /// Check if we're missing any events between oldest and highest
    pub fn detect_gaps(&self) -> Vec<u64> {
        if self.events.is_empty() {
            return vec![];
        }

        let oldest = self.events.front().unwrap().sequence;
        let mut missing = Vec::new();

        for seq in oldest..=self.highest_seen {
            if self.get(seq).is_none() {
                missing.push(seq);
            }
        }

        missing
    }

    /// Clear all events (for testing)
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.events.clear();
        self.highest_seen = 0;
    }

    /// Get event count
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event::DomainEvent;
    use uuid::Uuid;

    fn create_test_event(lobby_id: Uuid, sequence: u64) -> LobbyEvent {
        LobbyEvent::new(
            sequence,
            lobby_id,
            DomainEvent::GuestLeft {
                participant_id: Uuid::new_v4(),
            },
        )
    }

    #[test]
    fn test_append_assigns_sequence() {
        let mut log = EventLog::new();
        let lobby_id = Uuid::new_v4();

        let event = LobbyEvent::without_sequence(
            lobby_id,
            DomainEvent::LobbyCreated {
                lobby_id,
                host_id: Uuid::new_v4(),
                name: "Test".to_string(),
            },
        );

        let seq = log.append(event);

        assert_eq!(seq, 1);
        assert_eq!(log.next_sequence(), 2);
        assert_eq!(log.highest_sequence(), 1);
    }

    #[test]
    fn test_add_event_tracks_highest_seen() {
        let mut log = EventLog::new();
        let lobby_id = Uuid::new_v4();

        log.add_event(create_test_event(lobby_id, 5));
        assert_eq!(log.highest_sequence(), 5);

        log.add_event(create_test_event(lobby_id, 3));
        assert_eq!(log.highest_sequence(), 5); // Still 5

        log.add_event(create_test_event(lobby_id, 10));
        assert_eq!(log.highest_sequence(), 10);
    }

    #[test]
    fn test_get_since() {
        let mut log = EventLog::new();
        let lobby_id = Uuid::new_v4();

        for seq in 1..=5 {
            log.add_event(create_test_event(lobby_id, seq));
        }

        let since_3 = log.get_since(3);
        assert_eq!(since_3.len(), 2);
        assert_eq!(since_3[0].sequence, 4);
        assert_eq!(since_3[1].sequence, 5);
    }

    #[test]
    fn test_get_last() {
        let mut log = EventLog::new();
        let lobby_id = Uuid::new_v4();

        for seq in 1..=10 {
            log.add_event(create_test_event(lobby_id, seq));
        }

        let last_3 = log.get_last(3);
        assert_eq!(last_3.len(), 3);
        assert_eq!(last_3[0].sequence, 8);
        assert_eq!(last_3[1].sequence, 9);
        assert_eq!(last_3[2].sequence, 10);
    }

    #[test]
    fn test_bounded_buffer_evicts_oldest() {
        let mut log = EventLog::with_capacity(3);
        let lobby_id = Uuid::new_v4();

        for seq in 1..=5 {
            log.add_event(create_test_event(lobby_id, seq));
        }

        assert_eq!(log.len(), 3);
        assert!(log.get(1).is_none()); // Evicted
        assert!(log.get(2).is_none()); // Evicted
        assert!(log.get(3).is_some());
        assert!(log.get(4).is_some());
        assert!(log.get(5).is_some());
    }

    #[test]
    fn test_detect_gaps() {
        let mut log = EventLog::new();
        let lobby_id = Uuid::new_v4();

        log.add_event(create_test_event(lobby_id, 1));
        log.add_event(create_test_event(lobby_id, 2));
        log.add_event(create_test_event(lobby_id, 4)); // Gap at 3
        log.add_event(create_test_event(lobby_id, 7)); // Gaps at 5, 6

        let gaps = log.detect_gaps();
        assert_eq!(gaps, vec![3, 5, 6]);
    }

    #[test]
    fn test_detect_gaps_empty_log() {
        let log = EventLog::new();
        let gaps = log.detect_gaps();
        assert!(gaps.is_empty());
    }

    #[test]
    fn test_get_by_sequence() {
        let mut log = EventLog::new();
        let lobby_id = Uuid::new_v4();

        log.add_event(create_test_event(lobby_id, 5));
        log.add_event(create_test_event(lobby_id, 10));

        assert!(log.get(5).is_some());
        assert!(log.get(7).is_none());
        assert!(log.get(10).is_some());
    }
}
