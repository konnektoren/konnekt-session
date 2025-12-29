use konnekt_session_core::{Lobby, Participant};
use std::time::Instant;

/// Domain model for local session state
#[derive(Debug)]
pub struct SessionState {
    /// The lobby we're participating in
    lobby: Option<Lobby>,
    /// Our local participant
    local_participant: Participant,
    /// When we detected host disconnect (for grace period tracking)
    host_disconnect_time: Option<Instant>,
}

impl SessionState {
    pub fn new(participant: Participant) -> Self {
        Self {
            lobby: None,
            local_participant: participant,
            host_disconnect_time: None,
        }
    }

    pub fn participant(&self) -> &Participant {
        &self.local_participant
    }

    pub fn lobby(&self) -> Option<&Lobby> {
        self.lobby.as_ref()
    }

    pub fn lobby_mut(&mut self) -> Option<&mut Lobby> {
        self.lobby.as_mut()
    }

    pub fn set_lobby(&mut self, lobby: Lobby) {
        self.lobby = Some(lobby);
    }

    pub fn is_host(&self) -> bool {
        self.local_participant.is_host()
    }

    pub fn promote_to_host(&mut self) {
        self.local_participant.promote_to_host();
    }

    pub fn start_host_disconnect_timer(&mut self) {
        self.host_disconnect_time = Some(Instant::now());
    }

    pub fn clear_host_disconnect_timer(&mut self) {
        self.host_disconnect_time = None;
    }

    pub fn host_disconnect_elapsed(&self) -> Option<std::time::Duration> {
        self.host_disconnect_time.map(|t| t.elapsed())
    }
}
