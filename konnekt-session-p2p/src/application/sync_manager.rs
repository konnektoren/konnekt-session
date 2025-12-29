use crate::domain::{DomainEvent, EventLog, LobbyEvent, PeerId};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

/// Messages sent over the P2P network for event synchronization
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMessage {
    /// Host broadcasts an event to all guests
    EventBroadcast { event: LobbyEvent },

    /// Guest requests missing events (detected gap)
    RequestMissingEvents {
        lobby_id: Uuid,
        missing_sequences: Vec<u64>,
    },

    /// Host responds with requested events
    MissingEventsResponse { events: Vec<LobbyEvent> },

    /// Guest requests full sync (just joined)
    RequestFullSync {
        lobby_id: Uuid,
        last_known_sequence: u64, // 0 if just joined
    },

    /// Host responds with snapshot + recent events
    FullSyncResponse {
        snapshot: LobbySnapshot,
        events: Vec<LobbyEvent>, // Events after snapshot
    },
}

/// Snapshot of lobby state (for late joiners)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LobbySnapshot {
    pub lobby_id: Uuid,
    pub name: String,
    pub host_id: Uuid,
    pub participants: Vec<konnekt_session_core::Participant>,
    pub as_of_sequence: u64, // Snapshot represents state at this sequence
}

/// Manages event synchronization for a lobby
pub struct EventSyncManager {
    /// Our lobby ID
    lobby_id: Uuid,

    /// Are we the host?
    is_host: bool,

    /// Event log (bounded buffer)
    event_log: EventLog,

    /// Out-of-order events waiting for gaps to be filled
    pending_events: HashMap<u64, LobbyEvent>,

    /// Events we've requested but not received yet
    requested_sequences: Vec<u64>,
}

impl EventSyncManager {
    /// Create a new sync manager as host
    pub fn new_host(lobby_id: Uuid) -> Self {
        Self {
            lobby_id,
            is_host: true,
            event_log: EventLog::new(),
            pending_events: HashMap::new(),
            requested_sequences: Vec::new(),
        }
    }

    /// Create a new sync manager as guest
    pub fn new_guest(lobby_id: Uuid) -> Self {
        Self {
            lobby_id,
            is_host: false,
            event_log: EventLog::new(),
            pending_events: HashMap::new(),
            requested_sequences: Vec::new(),
        }
    }

    /// Promote to host (after delegation)
    pub fn promote_to_host(&mut self) {
        self.is_host = true;
    }

    /// Get current sequence number (for debugging)
    pub fn current_sequence(&self) -> u64 {
        if self.is_host {
            self.event_log.next_sequence() - 1
        } else {
            self.event_log.highest_sequence()
        }
    }

    /// Create and broadcast a new event (host only)
    pub fn create_event(&mut self, event: DomainEvent) -> Result<SyncMessage, SyncError> {
        if !self.is_host {
            return Err(SyncError::NotHost);
        }

        let lobby_event = LobbyEvent::without_sequence(self.lobby_id, event);
        let sequence = self.event_log.append(lobby_event.clone());

        tracing::debug!("Host created event sequence {}", sequence);

        Ok(SyncMessage::EventBroadcast {
            event: self.event_log.get(sequence).unwrap().clone(),
        })
    }

    /// Handle incoming sync message
    pub fn handle_message(
        &mut self,
        from: PeerId,
        message: SyncMessage,
    ) -> Result<SyncResponse, SyncError> {
        match message {
            SyncMessage::EventBroadcast { event } => self.handle_event_broadcast(event),

            SyncMessage::RequestMissingEvents {
                lobby_id,
                missing_sequences,
            } => self.handle_request_missing(lobby_id, missing_sequences),

            SyncMessage::MissingEventsResponse { events } => {
                self.handle_missing_events_response(events)
            }

            SyncMessage::RequestFullSync {
                lobby_id,
                last_known_sequence,
            } => {
                tracing::info!(
                    "Peer {} requested full sync from sequence {}",
                    from,
                    last_known_sequence
                );
                // Need snapshot from application layer
                Ok(SyncResponse::NeedSnapshot {
                    for_peer: from,
                    since_sequence: last_known_sequence,
                })
            }

            SyncMessage::FullSyncResponse { snapshot, events } => {
                self.handle_full_sync_response(snapshot, events)
            }
        }
    }

    /// Handle event broadcast from host
    fn handle_event_broadcast(&mut self, event: LobbyEvent) -> Result<SyncResponse, SyncError> {
        tracing::debug!("Received event broadcast: sequence {}", event.sequence);

        // Validate event is for our lobby
        if event.lobby_id != self.lobby_id {
            return Err(SyncError::WrongLobby);
        }

        let expected_sequence = self.event_log.highest_sequence() + 1;

        if event.sequence == expected_sequence {
            // Event is next in sequence - apply immediately
            self.event_log.add_event(event.clone());
            tracing::debug!("Applied event sequence {} immediately", event.sequence);

            // Try to apply any pending events that are now in sequence
            let applied_pending = self.try_apply_pending_events();

            // Check for gaps AFTER applying pending
            if let Some(response) = self.check_and_request_gaps()? {
                return Ok(response);
            }

            let mut events = vec![event];
            events.extend(applied_pending);

            Ok(SyncResponse::ApplyEvents { events })
        } else if event.sequence > expected_sequence {
            // Out of order - buffer it
            tracing::debug!(
                "Event {} is out of order (expected {}), buffering",
                event.sequence,
                expected_sequence
            );
            self.pending_events.insert(event.sequence, event);

            // Check for gaps including pending events
            if let Some(response) = self.check_and_request_gaps()? {
                return Ok(response);
            }

            Ok(SyncResponse::None)
        } else {
            // Duplicate or old event - ignore
            tracing::debug!(
                "Event {} is duplicate/old (expected {}), ignoring",
                event.sequence,
                expected_sequence
            );
            Ok(SyncResponse::None)
        }
    }

    /// Check for gaps and request them if needed
    fn check_and_request_gaps(&mut self) -> Result<Option<SyncResponse>, SyncError> {
        // Calculate what sequences we should have based on:
        // 1. Highest sequence in event log
        // 2. Highest sequence in pending events
        let highest_pending = self.pending_events.keys().max().copied().unwrap_or(0);
        let highest_overall = self.event_log.highest_sequence().max(highest_pending);

        if highest_overall == 0 {
            return Ok(None); // No events yet
        }

        // Find the oldest sequence we know about
        let oldest_in_log = if self.event_log.is_empty() {
            u64::MAX // No events in log yet
        } else {
            self.event_log.all_events()[0].sequence
        };

        let oldest_pending = self
            .pending_events
            .keys()
            .min()
            .copied()
            .unwrap_or(u64::MAX);
        let oldest = oldest_in_log.min(oldest_pending).min(1); // At minimum, start from 1

        let mut missing = Vec::new();
        for seq in oldest..=highest_overall {
            // Check if we have it in event log OR pending
            if self.event_log.get(seq).is_none() && !self.pending_events.contains_key(&seq) {
                missing.push(seq);
            }
        }

        if missing.is_empty() {
            return Ok(None);
        }

        // Only request if we haven't already requested these
        let new_missing: Vec<u64> = missing
            .iter()
            .filter(|seq| !self.requested_sequences.contains(seq))
            .copied()
            .collect();

        if new_missing.is_empty() {
            return Ok(None); // Already requested
        }

        tracing::warn!("Detected gaps in event log: {:?}", new_missing);
        self.requested_sequences.extend(&new_missing);

        Ok(Some(SyncResponse::SendMessage {
            to: None, // Broadcast to all (host will respond)
            message: SyncMessage::RequestMissingEvents {
                lobby_id: self.lobby_id,
                missing_sequences: new_missing,
            },
        }))
    }

    /// Try to apply pending events that are now in sequence
    fn try_apply_pending_events(&mut self) -> Vec<LobbyEvent> {
        let mut applied = Vec::new();

        loop {
            let next_expected = self.event_log.highest_sequence() + 1;

            if let Some(event) = self.pending_events.remove(&next_expected) {
                tracing::debug!(
                    "Applying pending event sequence {} from buffer",
                    event.sequence
                );
                self.event_log.add_event(event.clone());
                applied.push(event);
            } else {
                break;
            }
        }

        if !applied.is_empty() {
            tracing::info!("Applied {} pending events from buffer", applied.len());
        }

        applied
    }

    /// Handle request for missing events (host only)
    fn handle_request_missing(
        &mut self,
        lobby_id: Uuid,
        missing_sequences: Vec<u64>,
    ) -> Result<SyncResponse, SyncError> {
        if !self.is_host {
            return Err(SyncError::NotHost);
        }

        if lobby_id != self.lobby_id {
            return Err(SyncError::WrongLobby);
        }

        tracing::info!("Guest requested missing events: {:?}", missing_sequences);

        let mut events = Vec::new();
        for seq in missing_sequences {
            if let Some(event) = self.event_log.get(seq) {
                events.push(event.clone());
            } else {
                tracing::warn!("Guest requested sequence {} but we don't have it", seq);
            }
        }

        if events.is_empty() {
            return Ok(SyncResponse::None);
        }

        Ok(SyncResponse::SendMessage {
            to: None, // Will be sent back to requester
            message: SyncMessage::MissingEventsResponse { events },
        })
    }

    /// Handle response with missing events
    fn handle_missing_events_response(
        &mut self,
        events: Vec<LobbyEvent>,
    ) -> Result<SyncResponse, SyncError> {
        tracing::info!("Received {} missing events", events.len());

        let mut applied = Vec::new();

        for event in events {
            // Clear from requested list
            self.requested_sequences
                .retain(|&seq| seq != event.sequence);

            // Add to log
            self.event_log.add_event(event.clone());
            applied.push(event);
        }

        // Try to apply pending events
        let pending_applied = self.try_apply_pending_events();
        applied.extend(pending_applied);

        if !applied.is_empty() {
            Ok(SyncResponse::ApplyEvents { events: applied })
        } else {
            Ok(SyncResponse::None)
        }
    }

    /// Handle full sync response (late joiner)
    fn handle_full_sync_response(
        &mut self,
        snapshot: LobbySnapshot,
        events: Vec<LobbyEvent>,
    ) -> Result<SyncResponse, SyncError> {
        tracing::info!(
            "Received full sync: snapshot at sequence {}, {} events",
            snapshot.as_of_sequence,
            events.len()
        );

        // Clear our event log
        self.event_log = EventLog::new();

        // Add all events
        for event in &events {
            self.event_log.add_event(event.clone());
        }

        Ok(SyncResponse::ApplySnapshot {
            snapshot,
            events: events.clone(),
        })
    }

    /// Create a full sync response (host only)
    pub fn create_full_sync_response(
        &self,
        since_sequence: u64,
        snapshot: LobbySnapshot,
    ) -> Result<SyncMessage, SyncError> {
        if !self.is_host {
            return Err(SyncError::NotHost);
        }

        let events = if since_sequence == 0 {
            // New joiner - send all events we have
            self.event_log.all_events()
        } else {
            // Reconnecting - send events since last known
            self.event_log.get_since(since_sequence)
        };

        tracing::info!(
            "Creating full sync response: snapshot at {}, {} events",
            snapshot.as_of_sequence,
            events.len()
        );

        Ok(SyncMessage::FullSyncResponse { snapshot, events })
    }

    /// Request full sync from host (guest only)
    pub fn request_full_sync(&self) -> Result<SyncMessage, SyncError> {
        if self.is_host {
            return Err(SyncError::AlreadyHost);
        }

        Ok(SyncMessage::RequestFullSync {
            lobby_id: self.lobby_id,
            last_known_sequence: self.event_log.highest_sequence(),
        })
    }

    /// Get all events (for debugging)
    #[cfg(test)]
    pub fn all_events(&self) -> Vec<LobbyEvent> {
        self.event_log.all_events()
    }

    /// Get pending event count (for debugging)
    #[cfg(test)]
    pub fn pending_count(&self) -> usize {
        self.pending_events.len()
    }
}

/// Response from sync manager
#[derive(Debug)]
pub enum SyncResponse {
    /// No action needed
    None,

    /// Apply these events to domain state
    ApplyEvents { events: Vec<LobbyEvent> },

    /// Apply snapshot + events (full sync)
    ApplySnapshot {
        snapshot: LobbySnapshot,
        events: Vec<LobbyEvent>,
    },

    /// Send this message to peer(s)
    SendMessage {
        to: Option<PeerId>, // None = broadcast
        message: SyncMessage,
    },

    /// Application layer needs to provide snapshot
    NeedSnapshot {
        for_peer: PeerId,
        since_sequence: u64,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Not the host")]
    NotHost,

    #[error("Already the host")]
    AlreadyHost,

    #[error("Wrong lobby")]
    WrongLobby,

    #[error("Event out of order")]
    OutOfOrder,
}

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_core::Participant;

    fn create_test_event(sequence: u64, lobby_id: Uuid) -> LobbyEvent {
        LobbyEvent::new(
            sequence,
            lobby_id,
            DomainEvent::GuestLeft {
                participant_id: Uuid::new_v4(),
            },
        )
    }

    #[test]
    fn test_host_creates_events() {
        let lobby_id = Uuid::new_v4();
        let mut sync = EventSyncManager::new_host(lobby_id);

        let result = sync.create_event(DomainEvent::LobbyCreated {
            lobby_id,
            host_id: Uuid::new_v4(),
            name: "Test".to_string(),
        });

        assert!(result.is_ok());

        if let SyncMessage::EventBroadcast { event } = result.unwrap() {
            assert_eq!(event.sequence, 1);
            assert_eq!(event.lobby_id, lobby_id);
        } else {
            panic!("Expected EventBroadcast");
        }
    }

    #[test]
    fn test_guest_cannot_create_events() {
        let lobby_id = Uuid::new_v4();
        let mut sync = EventSyncManager::new_guest(lobby_id);

        let result = sync.create_event(DomainEvent::LobbyCreated {
            lobby_id,
            host_id: Uuid::new_v4(),
            name: "Test".to_string(),
        });

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SyncError::NotHost));
    }

    #[test]
    fn test_guest_applies_in_order_events() {
        let lobby_id = Uuid::new_v4();
        let mut sync = EventSyncManager::new_guest(lobby_id);
        let peer = PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()));

        // Apply events 1, 2, 3 in order
        for seq in 1..=3 {
            let msg = SyncMessage::EventBroadcast {
                event: create_test_event(seq, lobby_id),
            };

            let response = sync.handle_message(peer, msg).unwrap();

            match response {
                SyncResponse::ApplyEvents { events } => {
                    assert_eq!(events.len(), 1);
                    assert_eq!(events[0].sequence, seq);
                }
                _ => panic!("Expected ApplyEvents"),
            }
        }

        assert_eq!(sync.current_sequence(), 3);
    }

    #[test]
    fn test_guest_buffers_out_of_order_events() {
        let lobby_id = Uuid::new_v4();
        let mut sync = EventSyncManager::new_guest(lobby_id);
        let peer = PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()));

        // Receive event 3 first (out of order)
        let msg = SyncMessage::EventBroadcast {
            event: create_test_event(3, lobby_id),
        };

        let response = sync.handle_message(peer, msg).unwrap();

        // Should request missing events
        match response {
            SyncResponse::SendMessage { message, to } => {
                assert!(to.is_none()); // Broadcast
                if let SyncMessage::RequestMissingEvents {
                    missing_sequences, ..
                } = message
                {
                    // Should request 1 and 2
                    assert_eq!(missing_sequences.len(), 2);
                    assert!(missing_sequences.contains(&1));
                    assert!(missing_sequences.contains(&2));
                } else {
                    panic!("Expected RequestMissingEvents, got: {:?}", message);
                }
            }
            other => panic!("Expected SendMessage, got: {:?}", other),
        }

        assert_eq!(sync.pending_count(), 1); // Event 3 is buffered
        assert_eq!(sync.current_sequence(), 0); // Not applied yet
    }

    #[test]
    fn test_guest_applies_pending_after_gap_filled() {
        let lobby_id = Uuid::new_v4();
        let mut sync = EventSyncManager::new_guest(lobby_id);
        let peer = PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()));

        // Receive events 1, 3, 4 (missing 2)
        sync.handle_message(
            peer,
            SyncMessage::EventBroadcast {
                event: create_test_event(1, lobby_id),
            },
        )
        .unwrap();

        sync.handle_message(
            peer,
            SyncMessage::EventBroadcast {
                event: create_test_event(3, lobby_id),
            },
        )
        .unwrap();

        sync.handle_message(
            peer,
            SyncMessage::EventBroadcast {
                event: create_test_event(4, lobby_id),
            },
        )
        .unwrap();

        assert_eq!(sync.current_sequence(), 1); // Only 1 applied
        assert_eq!(sync.pending_count(), 2); // 3 and 4 buffered

        // Receive missing event 2
        let response = sync
            .handle_message(
                peer,
                SyncMessage::EventBroadcast {
                    event: create_test_event(2, lobby_id),
                },
            )
            .unwrap();

        // Should apply 2, 3, 4
        match response {
            SyncResponse::ApplyEvents { events } => {
                assert_eq!(events.len(), 3);
                assert_eq!(events[0].sequence, 2);
                assert_eq!(events[1].sequence, 3);
                assert_eq!(events[2].sequence, 4);
            }
            _ => panic!("Expected ApplyEvents"),
        }

        assert_eq!(sync.current_sequence(), 4);
        assert_eq!(sync.pending_count(), 0);
    }

    #[test]
    fn test_host_responds_to_missing_events_request() {
        let lobby_id = Uuid::new_v4();
        let mut sync = EventSyncManager::new_host(lobby_id);
        let peer = PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()));

        // Host creates events 1-5
        for _ in 1..=5 {
            sync.create_event(DomainEvent::GuestLeft {
                participant_id: Uuid::new_v4(),
            })
            .unwrap();
        }

        // Guest requests events 2 and 4
        let request = SyncMessage::RequestMissingEvents {
            lobby_id,
            missing_sequences: vec![2, 4],
        };

        let response = sync.handle_message(peer, request).unwrap();

        match response {
            SyncResponse::SendMessage { message, .. } => {
                if let SyncMessage::MissingEventsResponse { events } = message {
                    assert_eq!(events.len(), 2);
                    assert_eq!(events[0].sequence, 2);
                    assert_eq!(events[1].sequence, 4);
                } else {
                    panic!("Expected MissingEventsResponse");
                }
            }
            _ => panic!("Expected SendMessage"),
        }
    }

    #[test]
    fn test_full_sync_response() {
        let lobby_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();

        let snapshot = LobbySnapshot {
            lobby_id,
            name: "Test Lobby".to_string(),
            host_id,
            participants: vec![Participant::new_host("Host".to_string()).unwrap()],
            as_of_sequence: 10,
        };

        let events = vec![
            create_test_event(11, lobby_id),
            create_test_event(12, lobby_id),
        ];

        let mut sync = EventSyncManager::new_guest(lobby_id);
        let peer = PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()));

        let msg = SyncMessage::FullSyncResponse {
            snapshot: snapshot.clone(),
            events: events.clone(),
        };

        let response = sync.handle_message(peer, msg).unwrap();

        match response {
            SyncResponse::ApplySnapshot {
                snapshot: recv_snapshot,
                events: recv_events,
            } => {
                assert_eq!(recv_snapshot.as_of_sequence, 10);
                assert_eq!(recv_events.len(), 2);
            }
            _ => panic!("Expected ApplySnapshot"),
        }

        assert_eq!(sync.current_sequence(), 12);
    }

    #[test]
    fn test_promote_to_host() {
        let lobby_id = Uuid::new_v4();
        let mut sync = EventSyncManager::new_guest(lobby_id);

        assert!(!sync.is_host);

        // Try to create event as guest - should fail
        assert!(
            sync.create_event(DomainEvent::GuestLeft {
                participant_id: Uuid::new_v4()
            })
            .is_err()
        );

        // Promote to host
        sync.promote_to_host();

        assert!(sync.is_host);

        // Now should be able to create events
        assert!(
            sync.create_event(DomainEvent::GuestLeft {
                participant_id: Uuid::new_v4()
            })
            .is_ok()
        );
    }

    #[test]
    fn test_gap_detection_with_pending() {
        let lobby_id = Uuid::new_v4();
        let mut sync = EventSyncManager::new_guest(lobby_id);
        let peer = PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()));

        // Apply event 1
        sync.handle_message(
            peer,
            SyncMessage::EventBroadcast {
                event: create_test_event(1, lobby_id),
            },
        )
        .unwrap();

        // Buffer event 5 (missing 2, 3, 4)
        let response = sync
            .handle_message(
                peer,
                SyncMessage::EventBroadcast {
                    event: create_test_event(5, lobby_id),
                },
            )
            .unwrap();

        // Should detect gaps 2, 3, 4
        match response {
            SyncResponse::SendMessage { message, .. } => {
                if let SyncMessage::RequestMissingEvents {
                    missing_sequences, ..
                } = message
                {
                    assert_eq!(missing_sequences.len(), 3);
                    assert_eq!(missing_sequences, vec![2, 3, 4]);
                } else {
                    panic!("Expected RequestMissingEvents");
                }
            }
            _ => panic!("Expected SendMessage"),
        }
    }
}
