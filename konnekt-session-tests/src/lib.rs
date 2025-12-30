use cucumber::World;
use konnekt_session_core::{
    DomainCommand, DomainEvent, DomainEventLoop, Lobby, LobbyError, Participant,
};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, World, Default)]
pub struct SessionWorld {
    /// Domain event loop (the system under test)
    pub event_loop: DomainEventLoop,

    /// Last command executed (for debugging)
    pub last_command: Option<DomainCommand>,

    /// Last event emitted (for assertions)
    pub last_event: Option<DomainEvent>,

    /// Track lobby IDs by name for easy lookup
    pub lobby_ids: HashMap<String, Uuid>,

    /// Track participant IDs by name
    pub participant_ids: HashMap<String, Uuid>,

    /// Track errors (also used to temporarily store P2P events as JSON)
    pub last_error: Option<String>,
}

impl SessionWorld {
    /// Execute a command and store the result
    pub fn execute(&mut self, command: DomainCommand) -> &DomainEvent {
        self.last_command = Some(command.clone());
        let event = self.event_loop.handle_command(command);

        // Extract error if CommandFailed
        if let DomainEvent::CommandFailed { reason, .. } = &event {
            self.last_error = Some(reason.clone());
        }

        self.last_event = Some(event);
        self.last_event.as_ref().unwrap()
    }

    /// Get the last event (panics if none)
    pub fn last_event(&self) -> &DomainEvent {
        self.last_event.as_ref().expect("No event executed yet")
    }

    /// Get a lobby by name
    pub fn get_lobby(&self, lobby_name: &str) -> Option<&Lobby> {
        let lobby_id = self.lobby_ids.get(lobby_name)?;
        self.event_loop.get_lobby(lobby_id)
    }

    /// Get participant ID by name
    pub fn get_participant_id(&self, name: &str) -> Uuid {
        *self
            .participant_ids
            .get(name)
            .unwrap_or_else(|| panic!("Participant '{}' not found", name))
    }

    /// Check if last event was a failure
    pub fn last_command_failed(&self) -> bool {
        matches!(self.last_event, Some(DomainEvent::CommandFailed { .. }))
    }

    /// Get last error message
    pub fn last_error_message(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    // 🆕 Helper for P2P integration tests
    /// Get or create a lobby ID for P2P tests
    pub fn get_or_create_lobby_id(&mut self) -> Uuid {
        if self.lobby_ids.is_empty() {
            let lobby_id = Uuid::new_v4();
            self.lobby_ids.insert("Test Lobby".to_string(), lobby_id);
            lobby_id
        } else {
            *self.lobby_ids.values().next().unwrap()
        }
    }
}
