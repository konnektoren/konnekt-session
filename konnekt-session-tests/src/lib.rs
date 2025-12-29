use cucumber::World;
use konnekt_session_core::{Lobby, LobbyError, Participant};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, World, Default)]
pub struct SessionWorld {
    /// Current lobby under test
    pub lobby: Option<Lobby>,

    /// Participants by name for easy lookup
    pub participants_by_name: HashMap<String, Participant>,

    /// Last operation result
    pub last_error: Option<LobbyError>,

    /// Track when participants joined (for testing)
    pub join_times: HashMap<String, u64>,

    /// Simulated current time (milliseconds)
    pub current_time: u64,
}

impl SessionWorld {
    pub fn lobby(&self) -> &Lobby {
        self.lobby.as_ref().expect("No lobby created yet")
    }

    pub fn lobby_mut(&mut self) -> &mut Lobby {
        self.lobby.as_mut().expect("No lobby created yet")
    }

    pub fn get_participant(&self, name: &str) -> &Participant {
        self.participants_by_name
            .get(name)
            .unwrap_or_else(|| panic!("Participant '{}' not found", name))
    }

    pub fn get_participant_id(&self, name: &str) -> Uuid {
        self.get_participant(name).id()
    }

    pub fn advance_time(&mut self, ms: u64) {
        self.current_time += ms;
    }
}
