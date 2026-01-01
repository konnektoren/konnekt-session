use crate::domain::{
    ActivityId, ActivityMetadata, ActivityResult, ActivityStatus, Participant, ParticipantError,
    ParticipationMode, Timestamp,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Lobby aggregate root
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Lobby {
    /// Unique lobby identifier
    id: Uuid,

    /// Lobby name
    name: String,

    /// All participants (key: participant ID)
    participants: HashMap<Uuid, Participant>,

    /// Current host's participant ID
    host_id: Uuid,

    /// Planned and in-progress activities
    #[serde(default)]
    activities: Vec<ActivityMetadata>,

    /// Submitted results (for current/recent activities)
    #[serde(default)]
    activity_results: Vec<ActivityResult>,
}

/// Errors that can occur in lobby operations
#[derive(Debug, thiserror::Error, PartialEq, Serialize, Deserialize, JsonSchema)]
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

    // Activity errors
    #[error("Activity not found: {0}")]
    ActivityNotFound(ActivityId),

    #[error("Activity already exists: {0}")]
    ActivityAlreadyExists(ActivityId),

    #[error("Activity not in progress")]
    ActivityNotInProgress,

    #[error("Activity already in progress")]
    ActivityAlreadyInProgress,

    #[error("Spectator cannot submit results")]
    SpectatorCannotSubmit,

    #[error("Participant already submitted result for this activity")]
    DuplicateSubmission,
}

impl Lobby {
    /// Create a new lobby with random ID
    pub fn new(name: String, host: Participant) -> Result<Self, LobbyError> {
        Self::with_id(Uuid::new_v4(), name, host)
    }

    /// Create a new lobby with a specific ID
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
            activities: Vec::new(),
            activity_results: Vec::new(),
        })
    }

    // ===== Getters =====

    /// Get lobby ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get lobby name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get current host ID
    pub fn host_id(&self) -> Uuid {
        self.host_id
    }

    /// Get current host participant
    pub fn host(&self) -> Option<&Participant> {
        self.participants.get(&self.host_id)
    }

    /// Get all participants
    pub fn participants(&self) -> &HashMap<Uuid, Participant> {
        &self.participants
    }

    /// Get mutable access to participants (for removing timed out peers)
    pub fn participants_mut(&mut self) -> &mut HashMap<Uuid, Participant> {
        &mut self.participants
    }

    /// Get all activities
    pub fn activities(&self) -> &[ActivityMetadata] {
        &self.activities
    }

    /// Get activity by ID
    pub fn get_activity(&self, activity_id: ActivityId) -> Option<&ActivityMetadata> {
        self.activities.iter().find(|a| a.id == activity_id)
    }

    /// Get current in-progress activity
    pub fn current_activity(&self) -> Option<&ActivityMetadata> {
        self.activities
            .iter()
            .find(|a| a.status == ActivityStatus::InProgress)
    }

    /// Get results for an activity
    pub fn get_results(&self, activity_id: ActivityId) -> Vec<&ActivityResult> {
        self.activity_results
            .iter()
            .filter(|r| r.activity_id == activity_id)
            .collect()
    }

    // ===== Participant Management =====

    /// Add a guest to the lobby
    pub fn add_guest(&mut self, guest: Participant) -> Result<(), LobbyError> {
        if guest.is_host() {
            return Err(LobbyError::CannotDelegateToNonGuest);
        }

        // ✅ Idempotent: If participant already exists, just update it
        if let Some(existing) = self.participants.get(&guest.id()) {
            if existing.name() == guest.name() {
                tracing::debug!("Participant {} already exists, skipping", guest.id());
                return Ok(());
            }
        }

        self.participants.insert(guest.id(), guest);
        Ok(())
    }

    /// Remove a participant by ID
    /// For host timeout scenarios, use participants_mut().remove() directly
    pub fn remove_participant(&mut self, participant_id: Uuid) -> Result<bool, LobbyError> {
        // Don't allow removing the current host via this method
        // For timeout scenarios, caller should use participants_mut().remove()
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

    /// Kick a guest from the lobby (host only)
    pub fn kick_guest(&mut self, guest_id: Uuid, host_id: Uuid) -> Result<Participant, LobbyError> {
        // First, verify requester is host (immutable borrow)
        let requester = self
            .participants
            .get(&host_id)
            .ok_or(LobbyError::ParticipantNotFound(host_id))?;

        if !requester.is_host() {
            return Err(LobbyError::PermissionDenied);
        }

        // Cannot kick yourself (host)
        if guest_id == host_id {
            return Err(LobbyError::CannotKickHost);
        }

        // Drop immutable borrow before getting mutable borrow

        // Now remove the guest (mutable borrow)
        let kicked_participant = self
            .participants
            .remove(&guest_id)
            .ok_or(LobbyError::ParticipantNotFound(guest_id))?;

        // Verify they were actually a guest
        if kicked_participant.is_host() {
            // Put them back if we accidentally tried to remove host
            self.participants
                .insert(guest_id, kicked_participant.clone());
            return Err(LobbyError::CannotKickHost);
        }

        Ok(kicked_participant)
    }

    /// Check if there are any guests in the lobby
    pub fn has_guests(&self) -> bool {
        self.participants.values().any(|p| !p.is_host())
    }

    // ===== Host Delegation =====

    /// Manually delegate host role to a guest
    pub fn delegate_host(&mut self, new_host_id: Uuid) -> Result<(), LobbyError> {
        // Get the new host (must be a guest)
        let new_host = self
            .participants
            .get_mut(&new_host_id)
            .ok_or(LobbyError::ParticipantNotFound(new_host_id))?;

        if new_host.is_host() {
            return Err(LobbyError::CannotDelegateToNonGuest);
        }

        // Promote new host
        new_host.promote_to_host();

        // Demote old host (if they still exist in the map)
        // This handles the case where the old host timed out and was removed
        if let Some(old_host) = self.participants.get_mut(&self.host_id) {
            if old_host.id() != new_host_id {
                old_host.demote_to_guest();
            }
        }

        // Update host ID
        self.host_id = new_host_id;

        Ok(())
    }

    /// Automatically delegate host to the oldest guest (deterministic election)
    pub fn auto_delegate_host(&mut self) -> Result<Uuid, LobbyError> {
        // Find oldest guest (earliest join timestamp)
        // Important: Filter out the current host_id in case they're still in the map
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
            None => {
                // No guests available - lobby is empty except for (maybe) host
                Err(LobbyError::EmptyLobby)
            }
        }
    }

    // ===== Participation Mode Management =====

    /// Toggle participation mode for a guest (self-requested)
    pub fn toggle_guest_participation_mode(
        &mut self,
        guest_id: Uuid,
        activity_in_progress: bool,
    ) -> Result<ParticipationMode, LobbyError> {
        let participant = self
            .participants
            .get_mut(&guest_id)
            .ok_or(LobbyError::ParticipantNotFound(guest_id))?;

        participant
            .toggle_participation_mode(activity_in_progress)
            .map_err(Into::into)
    }

    /// Force a guest's participation mode (host action)
    pub fn force_guest_participation_mode(
        &mut self,
        guest_id: Uuid,
        mode: ParticipationMode,
    ) -> Result<(), LobbyError> {
        let participant = self
            .participants
            .get_mut(&guest_id)
            .ok_or(LobbyError::ParticipantNotFound(guest_id))?;

        participant.force_participation_mode(mode);
        Ok(())
    }

    /// Toggle participation mode for a participant
    /// Returns the new mode if successful
    pub fn toggle_participation_mode(
        &mut self,
        participant_id: Uuid,
        requester_id: Uuid,
        activity_in_progress: bool,
    ) -> Result<ParticipationMode, LobbyError> {
        // First, check permissions (immutable borrow)
        let requester = self
            .participants
            .get(&requester_id)
            .ok_or(LobbyError::ParticipantNotFound(requester_id))?;

        let is_self = participant_id == requester_id;
        let is_host = requester.is_host();

        if !is_self && !is_host {
            return Err(LobbyError::PermissionDenied);
        }

        // Drop immutable borrow before getting mutable borrow
        // (requester goes out of scope here)

        // Now get the participant to modify (mutable borrow)
        let participant = self
            .participants
            .get_mut(&participant_id)
            .ok_or(LobbyError::ParticipantNotFound(participant_id))?;

        // Toggle the mode
        participant
            .toggle_participation_mode(activity_in_progress)
            .map_err(LobbyError::from)
    }

    /// Force set participation mode (host only)
    pub fn force_participation_mode(
        &mut self,
        participant_id: Uuid,
        host_id: Uuid,
        mode: ParticipationMode,
    ) -> Result<(), LobbyError> {
        // First, verify requester is host (immutable borrow)
        let requester = self
            .participants
            .get(&host_id)
            .ok_or(LobbyError::ParticipantNotFound(host_id))?;

        if !requester.is_host() {
            return Err(LobbyError::PermissionDenied);
        }

        // Drop immutable borrow before getting mutable borrow

        // Now get the participant to modify (mutable borrow)
        let participant = self
            .participants
            .get_mut(&participant_id)
            .ok_or(LobbyError::ParticipantNotFound(participant_id))?;

        participant.force_participation_mode(mode);
        Ok(())
    }

    /// Get all active participants (excluding spectators)
    pub fn active_participants(&self) -> Vec<&Participant> {
        self.participants
            .values()
            .filter(|p| p.can_submit_results())
            .collect()
    }

    /// Get all spectating participants
    pub fn spectating_participants(&self) -> Vec<&Participant> {
        self.participants
            .values()
            .filter(|p| !p.can_submit_results())
            .collect()
    }

    // ===== Activity Management =====

    /// Host plans an activity
    pub fn plan_activity(&mut self, metadata: ActivityMetadata) -> Result<(), LobbyError> {
        // Validate
        if self.activities.iter().any(|a| a.id == metadata.id) {
            return Err(LobbyError::ActivityAlreadyExists(metadata.id));
        }

        // Add to queue
        self.activities.push(metadata);
        Ok(())
    }

    /// Host starts a planned activity
    pub fn start_activity(&mut self, activity_id: ActivityId) -> Result<(), LobbyError> {
        // Check only one activity can be in-progress (immutable borrow)
        let has_in_progress = self
            .activities
            .iter()
            .any(|a| a.status == ActivityStatus::InProgress);

        if has_in_progress {
            return Err(LobbyError::ActivityAlreadyInProgress);
        }

        // Now we can safely get mutable borrow
        let activity = self
            .activities
            .iter_mut()
            .find(|a| a.id == activity_id)
            .ok_or(LobbyError::ActivityNotFound(activity_id))?;

        // Transition state
        activity.status = ActivityStatus::InProgress;
        self.clear_results_for_activity(activity_id);

        Ok(())
    }

    /// Participant submits result
    pub fn submit_result(&mut self, result: ActivityResult) -> Result<(), LobbyError> {
        // Find activity
        let activity = self
            .activities
            .iter()
            .find(|a| a.id == result.activity_id)
            .ok_or(LobbyError::ActivityNotFound(result.activity_id))?;

        tracing::info!(
            "📥 Lobby: Received result from participant {} for activity {}",
            result.participant_id,
            result.activity_id
        );

        // Check activity is in progress
        if activity.status != ActivityStatus::InProgress {
            tracing::warn!(
                "⚠️  Activity {} is not in progress (status: {:?})",
                result.activity_id,
                activity.status
            );
            return Err(LobbyError::ActivityNotInProgress);
        }

        // Check participant is Active (not Spectating)
        let participant = self
            .participants()
            .get(&result.participant_id)
            .ok_or(LobbyError::ParticipantNotFound(result.participant_id))?;

        if !participant.can_submit_results() {
            tracing::warn!(
                "⚠️  Participant {} cannot submit (mode: {:?})",
                result.participant_id,
                participant.participation_mode()
            );
            return Err(LobbyError::SpectatorCannotSubmit);
        }

        // Store result
        self.activity_results.push(result.clone());

        tracing::info!(
            "✅ Result stored. Total results: {}",
            self.activity_results.len()
        );

        // Check if all active participants submitted
        let active_count = self.active_participants().len();
        let submitted_count = self
            .activity_results
            .iter()
            .filter(|r| r.activity_id == activity.id)
            .count();

        tracing::info!(
            "📊 Activity progress: {}/{} participants submitted",
            submitted_count,
            active_count
        );

        if self.all_active_participants_submitted(activity.id) {
            tracing::info!("🏁 All participants submitted! Completing activity");
            self.complete_activity(activity.id)?;
        }

        Ok(())
    }

    /// Cancel an in-progress activity
    pub fn cancel_activity(&mut self, activity_id: ActivityId) -> Result<(), LobbyError> {
        let activity = self
            .activities
            .iter_mut()
            .find(|a| a.id == activity_id)
            .ok_or(LobbyError::ActivityNotFound(activity_id))?;

        if activity.status != ActivityStatus::InProgress {
            return Err(LobbyError::ActivityNotInProgress);
        }

        activity.status = ActivityStatus::Cancelled;
        self.clear_results_for_activity(activity_id);

        Ok(())
    }

    /// Remove a planned activity
    pub fn remove_planned_activity(&mut self, activity_id: ActivityId) -> Result<(), LobbyError> {
        let index = self
            .activities
            .iter()
            .position(|a| a.id == activity_id)
            .ok_or(LobbyError::ActivityNotFound(activity_id))?;

        let activity = &self.activities[index];
        if activity.status != ActivityStatus::Planned {
            return Err(LobbyError::ActivityAlreadyInProgress);
        }

        self.activities.remove(index);
        Ok(())
    }

    // ===== Private Helper Methods =====

    /// Clear results for an activity
    fn clear_results_for_activity(&mut self, activity_id: ActivityId) {
        self.activity_results
            .retain(|r| r.activity_id != activity_id);
    }

    /// Check if all active participants have submitted results
    fn all_active_participants_submitted(&self, activity_id: ActivityId) -> bool {
        let active_count = self.active_participants().len();
        let submitted_count = self
            .activity_results
            .iter()
            .filter(|r| r.activity_id == activity_id)
            .count();

        active_count > 0 && submitted_count >= active_count
    }

    /// Complete an activity
    fn complete_activity(&mut self, activity_id: ActivityId) -> Result<(), LobbyError> {
        let activity = self
            .activities
            .iter_mut()
            .find(|a| a.id == activity_id)
            .ok_or(LobbyError::ActivityNotFound(activity_id))?;

        activity.status = ActivityStatus::Completed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::LobbyRole;

    // ===== Existing Participant Tests =====

    #[test]
    fn test_create_lobby() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let lobby = Lobby::new("Test Lobby".to_string(), host.clone()).unwrap();

        assert_eq!(lobby.name(), "Test Lobby");
        assert_eq!(lobby.host_id(), host.id());
        assert_eq!(lobby.participants().len(), 1);
    }

    #[test]
    fn test_cannot_create_lobby_with_guest() {
        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        let result = Lobby::new("Test Lobby".to_string(), guest);

        assert_eq!(result, Err(LobbyError::NoHost));
    }

    #[test]
    fn test_add_guest() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        lobby.add_guest(guest.clone()).unwrap();

        assert_eq!(lobby.participants().len(), 2);
        assert!(lobby.participants().contains_key(&guest.id()));
    }

    #[test]
    fn test_cannot_add_host_as_guest() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let another_host = Participant::new_host("Bob".to_string()).unwrap();
        let result = lobby.add_guest(another_host);

        assert_eq!(result, Err(LobbyError::CannotDelegateToNonGuest));
    }

    #[test]
    fn test_remove_guest() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap();

        lobby.remove_participant(guest_id).unwrap();

        assert_eq!(lobby.participants().len(), 1);
        assert!(!lobby.participants().contains_key(&guest_id));
    }

    #[test]
    fn test_cannot_remove_host() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let host_id = host.id();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let result = lobby.remove_participant(host_id);

        assert_eq!(result, Err(LobbyError::CannotRemoveHost));
    }

    #[test]
    fn test_manual_delegate_host() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let old_host_id = host.id();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap();

        lobby.delegate_host(guest_id).unwrap();

        // Bob is now host
        assert_eq!(lobby.host_id(), guest_id);
        assert!(lobby.participants().get(&guest_id).unwrap().is_host());

        // Alice is now guest
        assert!(!lobby.participants().get(&old_host_id).unwrap().is_host());
    }

    #[test]
    fn test_cannot_delegate_to_nonexistent_participant() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let fake_id = Uuid::new_v4();
        let result = lobby.delegate_host(fake_id);

        assert_eq!(result, Err(LobbyError::ParticipantNotFound(fake_id)));
    }

    #[test]
    fn test_auto_delegate_to_oldest_guest() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        // Add guests with specific timestamps
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

        // Auto-delegate (should pick Bob - oldest)
        let new_host_id = lobby.auto_delegate_host().unwrap();

        assert_eq!(new_host_id, bob_id);
        assert_eq!(lobby.host_id(), bob_id);
        assert!(lobby.participants().get(&bob_id).unwrap().is_host());
    }

    #[test]
    fn test_auto_delegate_with_no_guests() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let result = lobby.auto_delegate_host();

        assert_eq!(result, Err(LobbyError::EmptyLobby));
    }

    #[test]
    fn test_has_guests() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        assert!(!lobby.has_guests());

        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        lobby.add_guest(guest).unwrap();

        assert!(lobby.has_guests());
    }

    #[test]
    fn test_deterministic_election_multiple_guests() {
        let host = Participant::new_host("Host".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        // Add 3 guests in random order
        let carol = Participant::with_timestamp(
            "Carol".to_string(),
            LobbyRole::Guest,
            Timestamp::from_millis(150),
        )
        .unwrap();

        let alice = Participant::with_timestamp(
            "Alice".to_string(),
            LobbyRole::Guest,
            Timestamp::from_millis(100),
        )
        .unwrap();
        let alice_id = alice.id();

        let bob = Participant::with_timestamp(
            "Bob".to_string(),
            LobbyRole::Guest,
            Timestamp::from_millis(200),
        )
        .unwrap();

        lobby.add_guest(carol).unwrap();
        lobby.add_guest(alice).unwrap();
        lobby.add_guest(bob).unwrap();

        // Should always pick Alice (earliest timestamp)
        let new_host_id = lobby.auto_delegate_host().unwrap();

        assert_eq!(new_host_id, alice_id);
    }

    #[test]
    fn test_auto_delegate_after_host_removed() {
        let host = Participant::with_timestamp(
            "Host".to_string(),
            LobbyRole::Host,
            Timestamp::from_millis(100),
        )
        .unwrap();
        let host_id = host.id();

        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        // Add two guests
        let alice = Participant::with_timestamp(
            "Alice".to_string(),
            LobbyRole::Guest,
            Timestamp::from_millis(200),
        )
        .unwrap();
        let alice_id = alice.id();

        let bob = Participant::with_timestamp(
            "Bob".to_string(),
            LobbyRole::Guest,
            Timestamp::from_millis(300),
        )
        .unwrap();

        lobby.add_guest(alice).unwrap();
        lobby.add_guest(bob).unwrap();

        // Simulate host timeout: remove host from participants
        lobby.participants_mut().remove(&host_id);

        // Now try to auto-delegate
        let result = lobby.auto_delegate_host();

        assert!(result.is_ok());
        let new_host_id = result.unwrap();
        assert_eq!(new_host_id, alice_id);

        // Verify Alice is now host
        assert!(lobby.participants().get(&alice_id).unwrap().is_host());
        assert_eq!(lobby.host_id(), alice_id);
    }

    #[test]
    fn test_delegate_host_with_removed_old_host() {
        let host = Participant::with_timestamp(
            "Host".to_string(),
            LobbyRole::Host,
            Timestamp::from_millis(100),
        )
        .unwrap();
        let host_id = host.id();

        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let alice = Participant::with_timestamp(
            "Alice".to_string(),
            LobbyRole::Guest,
            Timestamp::from_millis(200),
        )
        .unwrap();
        let alice_id = alice.id();

        lobby.add_guest(alice).unwrap();

        // Remove the old host
        lobby.participants_mut().remove(&host_id);

        // Try to delegate to Alice
        let result = lobby.delegate_host(alice_id);

        assert!(result.is_ok());
        assert_eq!(lobby.host_id(), alice_id);
        assert!(lobby.participants().get(&alice_id).unwrap().is_host());
    }

    #[test]
    fn test_toggle_guest_participation_mode() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap();

        // Toggle to spectating
        let result = lobby.toggle_guest_participation_mode(guest_id, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ParticipationMode::Spectating);

        let participant = lobby.participants().get(&guest_id).unwrap();
        assert_eq!(
            participant.participation_mode(),
            ParticipationMode::Spectating
        );

        // Toggle back to active
        let result = lobby.toggle_guest_participation_mode(guest_id, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ParticipationMode::Active);
    }

    #[test]
    fn test_cannot_toggle_during_activity() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap();

        // Try to toggle during activity
        let result = lobby.toggle_guest_participation_mode(guest_id, true);

        assert!(matches!(
            result,
            Err(LobbyError::ParticipantError(
                ParticipantError::CannotToggleDuringActivity
            ))
        ));
    }

    #[test]
    fn test_force_guest_participation_mode() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap();

        lobby
            .force_guest_participation_mode(guest_id, ParticipationMode::Spectating)
            .unwrap();

        let participant = lobby.participants().get(&guest_id).unwrap();
        assert_eq!(
            participant.participation_mode(),
            ParticipationMode::Spectating
        );
    }

    #[test]
    fn test_active_participants_filter() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let guest1 = Participant::new_guest("Bob".to_string()).unwrap();
        let guest1_id = guest1.id();
        lobby.add_guest(guest1).unwrap();

        let guest2 = Participant::new_guest("Carol".to_string()).unwrap();
        lobby.add_guest(guest2).unwrap();

        // Toggle Bob to spectating
        lobby
            .toggle_guest_participation_mode(guest1_id, false)
            .unwrap();

        let active = lobby.active_participants();
        assert_eq!(active.len(), 2); // Alice (host) + Carol

        let spectating = lobby.spectating_participants();
        assert_eq!(spectating.len(), 1); // Bob
    }

    #[test]
    fn test_toggle_nonexistent_participant() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let fake_id = Uuid::new_v4();
        let result = lobby.toggle_guest_participation_mode(fake_id, false);

        assert_eq!(result, Err(LobbyError::ParticipantNotFound(fake_id)));
    }

    #[test]
    fn test_kick_guest() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let host_id = host.id();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        let guest_id = guest.id();
        lobby.add_guest(guest.clone()).unwrap();

        // Host kicks guest
        let kicked = lobby.kick_guest(guest_id, host_id).unwrap();

        assert_eq!(kicked.name(), "Bob");
        assert_eq!(lobby.participants().len(), 1); // Only host remains
        assert!(!lobby.participants().contains_key(&guest_id));
    }

    #[test]
    fn test_guest_cannot_kick() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let guest1 = Participant::new_guest("Bob".to_string()).unwrap();
        let guest1_id = guest1.id();
        lobby.add_guest(guest1).unwrap();

        let guest2 = Participant::new_guest("Carol".to_string()).unwrap();
        let guest2_id = guest2.id();
        lobby.add_guest(guest2).unwrap();

        // Guest1 tries to kick Guest2
        let result = lobby.kick_guest(guest2_id, guest1_id);

        assert_eq!(result, Err(LobbyError::PermissionDenied));
        assert_eq!(lobby.participants().len(), 3); // No one was kicked
    }

    #[test]
    fn test_cannot_kick_host() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let host_id = host.id();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let result = lobby.kick_guest(host_id, host_id);

        assert_eq!(result, Err(LobbyError::CannotKickHost));
        assert_eq!(lobby.participants().len(), 1);
    }

    #[test]
    fn test_kick_nonexistent_participant() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let host_id = host.id();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let fake_id = Uuid::new_v4();
        let result = lobby.kick_guest(fake_id, host_id);

        assert_eq!(result, Err(LobbyError::ParticipantNotFound(fake_id)));
    }

    // ===== Activity Tests =====

    #[test]
    fn test_plan_activity() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let metadata = ActivityMetadata::new(
            "trivia-quiz".to_string(),
            "Quiz 1".to_string(),
            serde_json::json!({}),
        );
        let activity_id = metadata.id;

        lobby.plan_activity(metadata).unwrap();

        assert_eq!(lobby.activities().len(), 1);
        assert_eq!(lobby.get_activity(activity_id).unwrap().name, "Quiz 1");
    }

    #[test]
    fn test_start_activity() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let metadata = ActivityMetadata::new(
            "trivia-quiz".to_string(),
            "Quiz 1".to_string(),
            serde_json::json!({}),
        );
        let activity_id = metadata.id;
        lobby.plan_activity(metadata).unwrap();

        lobby.start_activity(activity_id).unwrap();

        let activity = lobby.get_activity(activity_id).unwrap();
        assert_eq!(activity.status, ActivityStatus::InProgress);
    }

    #[test]
    fn test_cannot_start_two_activities() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let activity1 =
            ActivityMetadata::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        let activity2 =
            ActivityMetadata::new("quiz".to_string(), "Q2".to_string(), serde_json::json!({}));

        lobby.plan_activity(activity1.clone()).unwrap();
        lobby.plan_activity(activity2.clone()).unwrap();

        lobby.start_activity(activity1.id).unwrap();

        let result = lobby.start_activity(activity2.id);
        assert_eq!(result, Err(LobbyError::ActivityAlreadyInProgress));
    }

    #[test]
    fn test_submit_result() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let host_id = host.id();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let metadata =
            ActivityMetadata::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        let activity_id = metadata.id;

        lobby.plan_activity(metadata).unwrap();
        lobby.start_activity(activity_id).unwrap();

        let result = ActivityResult::new(activity_id, host_id).with_score(42);
        lobby.submit_result(result).unwrap();

        let results = lobby.get_results(activity_id);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].score, Some(42));
    }

    #[test]
    fn test_activity_completes_when_all_submit() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let host_id = host.id();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap();

        let metadata =
            ActivityMetadata::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        let activity_id = metadata.id;

        lobby.plan_activity(metadata).unwrap();
        lobby.start_activity(activity_id).unwrap();

        // Host submits
        lobby
            .submit_result(ActivityResult::new(activity_id, host_id))
            .unwrap();
        assert_eq!(
            lobby.get_activity(activity_id).unwrap().status,
            ActivityStatus::InProgress
        );

        // Guest submits → should complete
        lobby
            .submit_result(ActivityResult::new(activity_id, guest_id))
            .unwrap();
        assert_eq!(
            lobby.get_activity(activity_id).unwrap().status,
            ActivityStatus::Completed
        );
    }

    #[test]
    fn test_spectator_cannot_submit() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap();

        // Make Bob a spectator
        lobby
            .toggle_guest_participation_mode(guest_id, false)
            .unwrap();

        let metadata =
            ActivityMetadata::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        let activity_id = metadata.id;

        lobby.plan_activity(metadata).unwrap();
        lobby.start_activity(activity_id).unwrap();

        let result = ActivityResult::new(activity_id, guest_id);
        assert_eq!(
            lobby.submit_result(result),
            Err(LobbyError::SpectatorCannotSubmit)
        );
    }

    #[test]
    fn test_cancel_activity() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let metadata =
            ActivityMetadata::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        let activity_id = metadata.id;

        lobby.plan_activity(metadata).unwrap();
        lobby.start_activity(activity_id).unwrap();

        lobby.cancel_activity(activity_id).unwrap();

        assert_eq!(
            lobby.get_activity(activity_id).unwrap().status,
            ActivityStatus::Cancelled
        );
    }

    #[test]
    fn test_remove_planned_activity() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let metadata =
            ActivityMetadata::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        let activity_id = metadata.id;

        lobby.plan_activity(metadata).unwrap();
        lobby.remove_planned_activity(activity_id).unwrap();

        assert_eq!(lobby.activities().len(), 0);
    }

    #[test]
    fn test_cannot_remove_started_activity() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let metadata =
            ActivityMetadata::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        let activity_id = metadata.id;

        lobby.plan_activity(metadata).unwrap();
        lobby.start_activity(activity_id).unwrap();

        let result = lobby.remove_planned_activity(activity_id);
        assert_eq!(result, Err(LobbyError::ActivityAlreadyInProgress));
    }

    #[test]
    fn test_echo_activity_flow() {
        use crate::activities::{EchoChallenge, EchoResult};

        let host = Participant::new_host("Alice".to_string()).unwrap();
        let host_id = host.id();
        let mut lobby = Lobby::new("Test".to_string(), host).unwrap();

        // Create echo challenge
        let challenge = EchoChallenge::new("Hello Rust".to_string());
        let metadata = ActivityMetadata::new(
            EchoChallenge::activity_type().to_string(),
            "Echo Challenge".to_string(),
            challenge.to_config(),
        );
        let activity_id = metadata.id;

        // Plan and start
        lobby.plan_activity(metadata).unwrap();
        lobby.start_activity(activity_id).unwrap();

        // Submit result
        let result = EchoResult::new("Hello Rust".to_string(), 1500);
        let score = challenge.calculate_score(&result.response);

        lobby
            .submit_result(
                ActivityResult::new(activity_id, host_id)
                    .with_data(result.to_json())
                    .with_score(score)
                    .with_time(result.time_ms),
            )
            .unwrap();

        // Verify
        assert_eq!(
            lobby.get_activity(activity_id).unwrap().status,
            ActivityStatus::Completed
        );
        assert_eq!(lobby.get_results(activity_id)[0].score, Some(100));
    }
}
