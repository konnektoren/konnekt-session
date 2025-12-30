use crate::domain::{DomainEvent, EventLog, LobbyEvent, PeerId};
use konnekt_session_core::DomainCommand;
use std::collections::HashMap;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

/// Messages sent over the P2P network for event synchronization
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMessage {
    /// Guest → Host: Execute this domain command
    CommandRequest { command: DomainCommand },

    /// Host → All: Domain event happened (with sequence number)
    EventBroadcast { event: LobbyEvent },

    /// Guest → Host: I just joined, send me full state
    RequestFullSync { lobby_id: Uuid },

    /// Host → Guest: Here's the full state
    FullSyncResponse {
        snapshot: LobbySnapshot,
        events: Vec<LobbyEvent>,
    },
}

/// Snapshot of lobby state (for late joiners)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LobbySnapshot {
    pub lobby_id: Uuid,
    pub name: String,
    pub host_id: Uuid,
    pub participants: Vec<konnekt_session_core::Participant>,
    pub as_of_sequence: u64,
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
}

impl EventSyncManager {
    /// Create a new sync manager as host
    #[instrument(fields(lobby_id = %lobby_id))]
    pub fn new_host(lobby_id: Uuid) -> Self {
        info!("Creating EventSyncManager as HOST");
        Self {
            lobby_id,
            is_host: true,
            event_log: EventLog::new(),
            pending_events: HashMap::new(),
        }
    }

    /// Create a new sync manager as guest
    #[instrument(fields(lobby_id = %lobby_id))]
    pub fn new_guest(lobby_id: Uuid) -> Self {
        info!("Creating EventSyncManager as GUEST");
        Self {
            lobby_id,
            is_host: false,
            event_log: EventLog::new(),
            pending_events: HashMap::new(),
        }
    }

    /// Promote to host (after delegation)
    #[instrument(skip(self))]
    pub fn promote_to_host(&mut self) {
        info!("Promoting EventSyncManager to HOST");
        self.is_host = true;
    }

    /// Get current sequence number
    pub fn current_sequence(&self) -> u64 {
        if self.is_host {
            self.event_log.next_sequence() - 1
        } else {
            self.event_log.highest_sequence()
        }
    }

    /// Create and broadcast a new event (host only)
    #[instrument(skip(self, event), fields(
        event_type = ?std::mem::discriminant(&event),
        is_host = %self.is_host
    ))]
    pub fn create_event(&mut self, event: DomainEvent) -> Result<SyncMessage, SyncError> {
        if !self.is_host {
            warn!("Attempted to create event as guest");
            return Err(SyncError::NotHost);
        }

        let lobby_event = LobbyEvent::without_sequence(self.lobby_id, event);
        let sequence = self.event_log.append(lobby_event.clone());

        debug!(sequence = %sequence, "Host created new event");

        Ok(SyncMessage::EventBroadcast {
            event: self.event_log.get(sequence).unwrap().clone(),
        })
    }

    /// Handle incoming sync message
    #[instrument(skip(self, message), fields(
        from = %from,
        message_type = ?std::mem::discriminant(&message)
    ))]
    pub fn handle_message(
        &mut self,
        from: PeerId,
        message: SyncMessage,
    ) -> Result<SyncResponse, SyncError> {
        match message {
            SyncMessage::CommandRequest { command } => {
                if !self.is_host {
                    warn!("Guest received CommandRequest, ignoring");
                    return Ok(SyncResponse::None);
                }

                info!("HOST: Received command request from peer");
                Ok(SyncResponse::ProcessCommand { command })
            }

            SyncMessage::EventBroadcast { event } => self.handle_event_broadcast(event),

            SyncMessage::RequestFullSync { lobby_id } => {
                if lobby_id != self.lobby_id {
                    warn!(expected = %self.lobby_id, received = %lobby_id, "Wrong lobby ID");
                    return Err(SyncError::WrongLobby);
                }

                info!("Peer requested full sync");
                Ok(SyncResponse::NeedSnapshot {
                    for_peer: from,
                    since_sequence: 0,
                })
            }

            SyncMessage::FullSyncResponse { snapshot, events } => {
                self.handle_full_sync_response(snapshot, events)
            }
        }
    }

    /// Handle event broadcast from host
    #[instrument(skip(self, event), fields(
        sequence = %event.sequence,
        lobby_id = %event.lobby_id
    ))]
    fn handle_event_broadcast(&mut self, event: LobbyEvent) -> Result<SyncResponse, SyncError> {
        debug!("Received event broadcast");

        // Validate event is for our lobby
        if event.lobby_id != self.lobby_id {
            warn!("Event for wrong lobby, rejecting");
            return Err(SyncError::WrongLobby);
        }

        let expected_sequence = self.event_log.highest_sequence() + 1;

        if event.sequence == expected_sequence {
            // Event is next in sequence - apply immediately
            self.event_log.add_event(event.clone());
            debug!("Applied event immediately (in sequence)");

            // Try to apply any pending events that are now in sequence
            let applied_pending = self.try_apply_pending_events();

            let mut events = vec![event];
            events.extend(applied_pending);

            Ok(SyncResponse::ApplyEvents { events })
        } else if event.sequence > expected_sequence {
            // Out of order - buffer it
            warn!(
                expected = %expected_sequence,
                received = %event.sequence,
                gap_size = %(event.sequence - expected_sequence),
                "Event out of order, buffering"
            );
            self.pending_events.insert(event.sequence, event);
            Ok(SyncResponse::None)
        } else {
            // Duplicate or old event - ignore
            debug!(
                expected = %expected_sequence,
                received = %event.sequence,
                "Duplicate/old event, ignoring"
            );
            Ok(SyncResponse::None)
        }
    }

    /// Try to apply pending events that are now in sequence
    #[instrument(skip(self), fields(
        pending_count = %self.pending_events.len()
    ))]
    fn try_apply_pending_events(&mut self) -> Vec<LobbyEvent> {
        let mut applied = Vec::new();

        loop {
            let next_expected = self.event_log.highest_sequence() + 1;

            if let Some(event) = self.pending_events.remove(&next_expected) {
                debug!(sequence = %event.sequence, "Applying pending event from buffer");
                self.event_log.add_event(event.clone());
                applied.push(event);
            } else {
                break;
            }
        }

        if !applied.is_empty() {
            info!(
                applied = %applied.len(),
                still_pending = %self.pending_events.len(),
                "Applied pending events from buffer"
            );
        }

        applied
    }

    /// Handle full sync response (late joiner)
    #[instrument(skip(self, snapshot, events), fields(
        snapshot.sequence = %snapshot.as_of_sequence,
        events_count = %events.len()
    ))]
    fn handle_full_sync_response(
        &mut self,
        snapshot: LobbySnapshot,
        events: Vec<LobbyEvent>,
    ) -> Result<SyncResponse, SyncError> {
        info!("Received full sync response");

        // Clear our event log
        self.event_log = EventLog::new();

        // Add all events
        for event in &events {
            self.event_log.add_event(event.clone());
        }

        debug!(
            final_sequence = %self.event_log.highest_sequence(),
            "Full sync applied"
        );

        // Create lobby from snapshot
        let create_lobby_event = DomainEvent::LobbyCreated {
            lobby_id: snapshot.lobby_id,
            host_id: snapshot.host_id,
            name: snapshot.name.clone(),
        };

        let lobby_event = LobbyEvent::new(0, snapshot.lobby_id, create_lobby_event);

        let mut all_events = vec![lobby_event];
        all_events.extend(events.clone());

        Ok(SyncResponse::ApplySnapshot {
            snapshot,
            events: all_events,
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
            self.event_log.all_events()
        } else {
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
        to: Option<PeerId>,
        message: SyncMessage,
    },

    /// Application layer needs to provide snapshot
    NeedSnapshot {
        for_peer: PeerId,
        since_sequence: u64,
    },

    /// Host should process this command locally
    ProcessCommand { command: DomainCommand },
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

    fn create_test_command() -> DomainCommand {
        DomainCommand::JoinLobby {
            lobby_id: Uuid::new_v4(),
            guest_name: "Alice".to_string(),
        }
    }

    #[test]
    fn test_host_receives_command_request() {
        let lobby_id = Uuid::new_v4();
        let mut sync = EventSyncManager::new_host(lobby_id);
        let peer = PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()));

        let msg = SyncMessage::CommandRequest {
            command: create_test_command(),
        };

        let response = sync.handle_message(peer, msg).unwrap();

        match response {
            SyncResponse::ProcessCommand { command } => {
                assert!(matches!(command, DomainCommand::JoinLobby { .. }));
            }
            _ => panic!("Expected ProcessCommand"),
        }
    }

    #[test]
    fn test_guest_ignores_command_request() {
        let lobby_id = Uuid::new_v4();
        let mut sync = EventSyncManager::new_guest(lobby_id);
        let peer = PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()));

        let msg = SyncMessage::CommandRequest {
            command: create_test_command(),
        };

        let response = sync.handle_message(peer, msg).unwrap();

        match response {
            SyncResponse::None => {} // Expected
            _ => panic!("Guest should ignore CommandRequest"),
        }
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
    fn test_guest_applies_in_order_events() {
        let lobby_id = Uuid::new_v4();
        let mut sync = EventSyncManager::new_guest(lobby_id);
        let peer = PeerId::new(matchbox_socket::PeerId(Uuid::new_v4()));

        for seq in 1..=3 {
            let event = LobbyEvent::new(
                seq,
                lobby_id,
                DomainEvent::GuestLeft {
                    participant_id: Uuid::new_v4(),
                },
            );

            let msg = SyncMessage::EventBroadcast { event };
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
}
