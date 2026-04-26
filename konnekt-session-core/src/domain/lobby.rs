use crate::domain::{
    ActivityConfig, ActivityId, ActivityRunId, Participant, ParticipantError, ParticipationMode,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lobby {
    id: Uuid,
    name: String,
    participants: HashMap<Uuid, Participant>,
    host_id: Uuid,
    activity_queue: Vec<ActivityConfig>,
    /// Some while a run is InProgress, None when idle.
    active_run_id: Option<ActivityRunId>,
}

#[derive(Debug, thiserror::Error, PartialEq, Serialize, Deserialize)]
pub enum LobbyError {
    #[error("Lobby must have exactly one host")]
    NoHost,

    #[error("Participant not found: {0}")]
    ParticipantNotFound(Uuid),

    #[error("Cannot delegate to non-guest participant")]
    CannotDelegateToNonGuest,

    #[error("Lobby is empty, cannot delegate host")]
    EmptyLobby,

    #[error("Cannot remove host without delegation")]
    CannotRemoveHost,

    #[error("Cannot kick the host")]
    CannotKickHost,

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Participant error: {0}")]
    ParticipantError(#[from] ParticipantError),

    #[error("Activity not found: {0}")]
    ActivityNotFound(ActivityId),

    #[error("Activity already exists: {0}")]
    ActivityAlreadyExists(ActivityId),

    #[error("A run is already in progress")]
    RunAlreadyInProgress,

    #[error("No run in progress")]
    NoRunInProgress,

    #[error("Activity queue is empty")]
    EmptyQueue,
}

impl Lobby {
    pub fn new(name: String, host: Participant) -> Result<Self, LobbyError> {
        Self::with_id(Uuid::new_v4(), name, host)
    }

    pub fn with_id(id: Uuid, name: String, host: Participant) -> Result<Self, LobbyError> {
        if !host.is_host() {
            return Err(LobbyError::NoHost);
        }
        let host_id = host.id();
        let mut participants = HashMap::new();
        participants.insert(host_id, host);

        Ok(Lobby {
            id,
            name,
            participants,
            host_id,
            activity_queue: Vec::new(),
            active_run_id: None,
        })
    }

    // ===== Getters =====

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn host_id(&self) -> Uuid {
        self.host_id
    }
    pub fn host(&self) -> Option<&Participant> {
        self.participants.get(&self.host_id)
    }
    pub fn participants(&self) -> &HashMap<Uuid, Participant> {
        &self.participants
    }
    pub fn participants_mut(&mut self) -> &mut HashMap<Uuid, Participant> {
        &mut self.participants
    }
    pub fn activity_queue(&self) -> &[ActivityConfig] {
        &self.activity_queue
    }
    pub fn active_run_id(&self) -> Option<ActivityRunId> {
        self.active_run_id
    }
    pub fn has_active_run(&self) -> bool {
        self.active_run_id.is_some()
    }

    // ===== Participant Management =====

    pub fn add_guest(&mut self, guest: Participant) -> Result<(), LobbyError> {
        if guest.is_host() {
            return Err(LobbyError::CannotDelegateToNonGuest);
        }
        if let Some(existing) = self.participants.get(&guest.id())
            && existing.name() == guest.name()
        {
            return Ok(());
        }
        self.participants.insert(guest.id(), guest);
        Ok(())
    }

    pub fn remove_participant(&mut self, participant_id: Uuid) -> Result<bool, LobbyError> {
        if participant_id == self.host_id {
            return Err(LobbyError::CannotRemoveHost);
        }
        let was_host = self
            .participants
            .get(&participant_id)
            .map(|p| p.is_host())
            .unwrap_or(false);
        self.participants
            .remove(&participant_id)
            .ok_or(LobbyError::ParticipantNotFound(participant_id))?;
        Ok(was_host)
    }

    pub fn kick_guest(&mut self, guest_id: Uuid, host_id: Uuid) -> Result<Participant, LobbyError> {
        let requester = self
            .participants
            .get(&host_id)
            .ok_or(LobbyError::ParticipantNotFound(host_id))?;
        if !requester.is_host() {
            return Err(LobbyError::PermissionDenied);
        }
        if guest_id == host_id {
            return Err(LobbyError::CannotKickHost);
        }
        let kicked = self
            .participants
            .remove(&guest_id)
            .ok_or(LobbyError::ParticipantNotFound(guest_id))?;
        if kicked.is_host() {
            self.participants.insert(guest_id, kicked.clone());
            return Err(LobbyError::CannotKickHost);
        }
        Ok(kicked)
    }

    pub fn has_guests(&self) -> bool {
        self.participants.values().any(|p| !p.is_host())
    }

    // ===== Host Delegation =====

    pub fn delegate_host(&mut self, new_host_id: Uuid) -> Result<(), LobbyError> {
        let new_host = self
            .participants
            .get_mut(&new_host_id)
            .ok_or(LobbyError::ParticipantNotFound(new_host_id))?;
        if new_host.is_host() {
            return Err(LobbyError::CannotDelegateToNonGuest);
        }
        new_host.promote_to_host();
        if let Some(old_host) = self.participants.get_mut(&self.host_id)
            && old_host.id() != new_host_id
        {
            old_host.demote_to_guest();
        }
        self.host_id = new_host_id;
        Ok(())
    }

    pub fn auto_delegate_host(&mut self) -> Result<Uuid, LobbyError> {
        let oldest_guest = self
            .participants
            .values()
            .filter(|p| !p.is_host() && p.id() != self.host_id)
            .min_by_key(|p| p.joined_at());
        match oldest_guest {
            Some(guest) => {
                let new_host_id = guest.id();
                self.delegate_host(new_host_id)?;
                Ok(new_host_id)
            }
            None => Err(LobbyError::EmptyLobby),
        }
    }

    // ===== Participation Mode =====

    pub fn toggle_participation_mode(
        &mut self,
        participant_id: Uuid,
        requester_id: Uuid,
    ) -> Result<ParticipationMode, LobbyError> {
        let requester = self
            .participants
            .get(&requester_id)
            .ok_or(LobbyError::ParticipantNotFound(requester_id))?;
        let is_self = participant_id == requester_id;
        let is_host = requester.is_host();
        if !is_self && !is_host {
            return Err(LobbyError::PermissionDenied);
        }
        let activity_in_progress = self.active_run_id.is_some();
        let participant = self
            .participants
            .get_mut(&participant_id)
            .ok_or(LobbyError::ParticipantNotFound(participant_id))?;
        participant
            .toggle_participation_mode(activity_in_progress)
            .map_err(LobbyError::from)
    }

    pub fn force_participation_mode(
        &mut self,
        participant_id: Uuid,
        host_id: Uuid,
        mode: ParticipationMode,
    ) -> Result<(), LobbyError> {
        let requester = self
            .participants
            .get(&host_id)
            .ok_or(LobbyError::ParticipantNotFound(host_id))?;
        if !requester.is_host() {
            return Err(LobbyError::PermissionDenied);
        }
        let participant = self
            .participants
            .get_mut(&participant_id)
            .ok_or(LobbyError::ParticipantNotFound(participant_id))?;
        participant.force_participation_mode(mode);
        Ok(())
    }

    pub fn active_participants(&self) -> Vec<&Participant> {
        self.participants
            .values()
            .filter(|p| p.can_submit_results())
            .collect()
    }

    /// Snapshot of active participant IDs — used when creating an ActivityRun.
    pub fn active_participant_ids(&self) -> HashSet<Uuid> {
        self.participants
            .values()
            .filter(|p| p.can_submit_results())
            .map(|p| p.id())
            .collect()
    }

    // ===== Activity Queue =====

    pub fn queue_activity(&mut self, config: ActivityConfig) -> Result<(), LobbyError> {
        if self.activity_queue.iter().any(|a| a.id == config.id) {
            return Err(LobbyError::ActivityAlreadyExists(config.id));
        }
        self.activity_queue.push(config);
        Ok(())
    }

    pub fn remove_queued_activity(&mut self, activity_id: ActivityId) -> Result<(), LobbyError> {
        let pos = self
            .activity_queue
            .iter()
            .position(|a| a.id == activity_id)
            .ok_or(LobbyError::ActivityNotFound(activity_id))?;
        self.activity_queue.remove(pos);
        Ok(())
    }

    /// Dequeue the next activity config. Returns it so caller can create an ActivityRun.
    pub fn dequeue_next_activity(&mut self) -> Result<ActivityConfig, LobbyError> {
        if self.active_run_id.is_some() {
            return Err(LobbyError::RunAlreadyInProgress);
        }
        if self.activity_queue.is_empty() {
            return Err(LobbyError::EmptyQueue);
        }
        Ok(self.activity_queue.remove(0))
    }

    pub fn set_active_run(&mut self, run_id: ActivityRunId) -> Result<(), LobbyError> {
        if self.active_run_id.is_some() {
            return Err(LobbyError::RunAlreadyInProgress);
        }
        self.active_run_id = Some(run_id);
        Ok(())
    }

    pub fn clear_active_run(&mut self) {
        self.active_run_id = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{LobbyRole, Timestamp};

    #[test]
    fn test_create_lobby() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let lobby = Lobby::new("Test Lobby".to_string(), host.clone()).unwrap();
        assert_eq!(lobby.name(), "Test Lobby");
        assert_eq!(lobby.host_id(), host.id());
        assert_eq!(lobby.participants().len(), 1);
        assert!(!lobby.has_active_run());
    }

    #[test]
    fn test_cannot_create_lobby_with_guest() {
        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        assert_eq!(
            Lobby::new("Test".to_string(), guest),
            Err(LobbyError::NoHost)
        );
    }

    #[test]
    fn test_add_guest() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test".to_string(), host).unwrap();
        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        lobby.add_guest(guest.clone()).unwrap();
        assert_eq!(lobby.participants().len(), 2);
    }

    #[test]
    fn test_kick_guest() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let host_id = host.id();
        let mut lobby = Lobby::new("Test".to_string(), host).unwrap();
        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap();
        lobby.kick_guest(guest_id, host_id).unwrap();
        assert_eq!(lobby.participants().len(), 1);
    }

    #[test]
    fn test_manual_delegate_host() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let old_host_id = host.id();
        let mut lobby = Lobby::new("Test".to_string(), host).unwrap();
        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap();
        lobby.delegate_host(guest_id).unwrap();
        assert_eq!(lobby.host_id(), guest_id);
        assert!(!lobby.participants().get(&old_host_id).unwrap().is_host());
    }

    #[test]
    fn test_auto_delegate_to_oldest_guest() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test".to_string(), host).unwrap();

        let bob = Participant::with_timestamp(
            "Bob".to_string(),
            LobbyRole::Guest,
            Timestamp::from_millis(100),
        )
        .unwrap();
        let bob_id = bob.id();
        let carol = Participant::with_timestamp(
            "Carol".to_string(),
            LobbyRole::Guest,
            Timestamp::from_millis(200),
        )
        .unwrap();

        lobby.add_guest(bob).unwrap();
        lobby.add_guest(carol).unwrap();

        let new_host_id = lobby.auto_delegate_host().unwrap();
        assert_eq!(new_host_id, bob_id);
    }

    #[test]
    fn test_active_participant_ids_snapshot() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let host_id = host.id();
        let mut lobby = Lobby::new("Test".to_string(), host).unwrap();
        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap();

        let snapshot = lobby.active_participant_ids();
        assert!(snapshot.contains(&host_id));
        assert!(snapshot.contains(&guest_id));
        assert_eq!(snapshot.len(), 2);
    }

    #[test]
    fn test_cannot_toggle_during_active_run() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let host_id = host.id();
        let mut lobby = Lobby::new("Test".to_string(), host).unwrap();
        lobby.set_active_run(Uuid::new_v4()).unwrap();

        let result = lobby.toggle_participation_mode(host_id, host_id);
        assert!(matches!(
            result,
            Err(LobbyError::ParticipantError(
                ParticipantError::CannotToggleDuringActivity
            ))
        ));
    }

    #[test]
    fn test_dequeue_activity() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test".to_string(), host).unwrap();
        let config =
            ActivityConfig::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        let config_id = config.id;
        lobby.queue_activity(config).unwrap();

        let dequeued = lobby.dequeue_next_activity().unwrap();
        assert_eq!(dequeued.id, config_id);
        assert!(lobby.activity_queue().is_empty());
    }

    #[test]
    fn test_cannot_dequeue_during_active_run() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test".to_string(), host).unwrap();
        let config =
            ActivityConfig::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        lobby.queue_activity(config).unwrap();
        lobby.set_active_run(Uuid::new_v4()).unwrap();

        assert_eq!(
            lobby.dequeue_next_activity(),
            Err(LobbyError::RunAlreadyInProgress)
        );
    }

    #[test]
    fn test_clear_active_run() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test".to_string(), host).unwrap();
        lobby.set_active_run(Uuid::new_v4()).unwrap();
        assert!(lobby.has_active_run());
        lobby.clear_active_run();
        assert!(!lobby.has_active_run());
    }
}
