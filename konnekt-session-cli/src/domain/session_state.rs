use konnekt_session_core::{Lobby, Participant, ParticipationMode};
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
    /// Whether an activity is currently in progress
    activity_in_progress: bool, // NEW
}

impl SessionState {
    pub fn new(participant: Participant) -> Self {
        Self {
            lobby: None,
            local_participant: participant,
            host_disconnect_time: None,
            activity_in_progress: false, // NEW
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

    // NEW: Activity state management
    pub fn is_activity_in_progress(&self) -> bool {
        self.activity_in_progress
    }

    pub fn set_activity_in_progress(&mut self, in_progress: bool) {
        self.activity_in_progress = in_progress;
    }

    // NEW: Toggle participation mode
    pub fn toggle_participation_mode(&mut self) -> Result<ParticipationMode, String> {
        if self.activity_in_progress {
            return Err("Cannot change mode during activity".to_string());
        }

        let new_mode = match self.local_participant.participation_mode() {
            ParticipationMode::Active => ParticipationMode::Spectating,
            ParticipationMode::Spectating => ParticipationMode::Active,
        };

        self.local_participant.force_participation_mode(new_mode);

        // Also update in lobby if we have one
        if let Some(lobby) = &mut self.lobby {
            let participant_id = self.local_participant.id();
            if let Some(participant) = lobby.participants_mut().get_mut(&participant_id) {
                participant.force_participation_mode(new_mode);
            }
        }

        Ok(new_mode)
    }
}
