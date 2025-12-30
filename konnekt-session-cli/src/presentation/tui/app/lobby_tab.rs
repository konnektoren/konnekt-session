use crossterm::event::KeyCode;
use konnekt_session_core::Lobby;

/// Lobby tab state (presentation only)
pub struct LobbyTab {
    lobby_name: Option<String>,
    participant_count: usize,
}

impl LobbyTab {
    pub fn new() -> Self {
        Self {
            lobby_name: None,
            participant_count: 0,
        }
    }

    pub fn handle_key(
        &mut self,
        _key: KeyCode,
    ) -> Option<crate::presentation::tui::app::UserAction> {
        None // Lobby tab is read-only
    }

    pub fn update_lobby(&mut self, lobby: &Lobby) {
        self.lobby_name = Some(lobby.name().to_string());
        self.participant_count = lobby.participants().len();
    }

    pub fn lobby_name(&self) -> Option<&str> {
        self.lobby_name.as_deref()
    }

    pub fn participant_count(&self) -> usize {
        self.participant_count
    }
}
