use konnekt_session_core::{Lobby, Participant};
use konnekt_session_p2p::PeerId;
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

/// Domain model for local session state
#[derive(Debug)]
pub struct SessionState {
    /// The lobby we're participating in
    lobby: Option<Lobby>,
    /// Our local participant
    local_participant: Participant,
    /// Map peer IDs to participant UUIDs
    peer_to_participant: HashMap<PeerId, Uuid>,
    /// When we detected host disconnect (for grace period tracking)
    host_disconnect_time: Option<Instant>,
}

impl SessionState {
    pub fn new(participant: Participant) -> Self {
        Self {
            lobby: None,
            local_participant: participant,
            peer_to_participant: HashMap::new(),
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

    pub fn add_peer_mapping(&mut self, peer_id: PeerId, participant_id: Uuid) {
        self.peer_to_participant.insert(peer_id, participant_id);
    }

    pub fn remove_peer_mapping(&mut self, peer_id: &PeerId) -> Option<Uuid> {
        self.peer_to_participant.remove(peer_id)
    }

    pub fn get_participant_id(&self, peer_id: &PeerId) -> Option<Uuid> {
        self.peer_to_participant.get(peer_id).copied()
    }

    pub fn is_peer_host(&self, peer_id: &PeerId) -> bool {
        if let Some(lobby) = &self.lobby {
            let host_id = lobby.host_id();
            self.peer_to_participant
                .get(peer_id)
                .map(|participant_id| *participant_id == host_id)
                .unwrap_or(false)
        } else {
            false
        }
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

    pub fn is_host_connected(&self) -> bool {
        if let Some(lobby) = &self.lobby {
            let host_id = lobby.host_id();
            self.peer_to_participant
                .values()
                .any(|participant_id| *participant_id == host_id)
        } else {
            false
        }
    }
}
