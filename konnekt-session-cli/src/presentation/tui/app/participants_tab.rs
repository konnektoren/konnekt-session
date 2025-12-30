use crossterm::event::KeyCode;
use konnekt_session_core::Lobby;
use uuid::Uuid;

use crate::presentation::tui::app::UserAction;

/// Participants tab state (presentation only)
pub struct ParticipantsTab {
    selected_participant: usize,
}

impl ParticipantsTab {
    pub fn new() -> Self {
        Self {
            selected_participant: 0,
        }
    }

    pub fn handle_key(
        &mut self,
        key: KeyCode,
        is_host: bool,
        lobby: &Option<Lobby>,
    ) -> Option<UserAction> {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(lobby) = lobby {
                    let max = lobby.participants().len().saturating_sub(1);
                    self.selected_participant = (self.selected_participant + 1).min(max);
                }
                None
            }

            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_participant = self.selected_participant.saturating_sub(1);
                None
            }

            KeyCode::Char('t') => Some(UserAction::ToggleParticipationMode),

            KeyCode::Char('x') if is_host => {
                if let Some(lobby) = lobby {
                    let participants: Vec<_> = lobby.participants().values().collect();
                    if self.selected_participant < participants.len() {
                        let selected = participants[self.selected_participant];
                        if !selected.is_host() {
                            return Some(UserAction::KickParticipant(selected.id()));
                        }
                    }
                }
                None
            }

            _ => None,
        }
    }

    pub fn update_lobby(&mut self, lobby: &Lobby) {
        // Reset selection if out of bounds
        let max = lobby.participants().len().saturating_sub(1);
        self.selected_participant = self.selected_participant.min(max);
    }

    pub fn selected_participant(&self) -> usize {
        self.selected_participant
    }
}
