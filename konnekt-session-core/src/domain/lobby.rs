use crate::domain::{Participant, ParticipantError, Timestamp};
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

    #[error("Participant error: {0}")]
    ParticipantError(#[from] ParticipantError),
}

impl Lobby {
    /// Create a new lobby with a host
    pub fn new(name: String, host: Participant) -> Result<Self, LobbyError> {
        if !host.is_host() {
            return Err(LobbyError::NoHost);
        }

        let id = Uuid::new_v4();
        let host_id = host.id();
        let mut participants = HashMap::new();
        participants.insert(host_id, host);

        Ok(Lobby {
            id,
            name,
            participants,
            host_id,
        })
    }

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

    /// Add a guest to the lobby
    pub fn add_guest(&mut self, guest: Participant) -> Result<(), LobbyError> {
        if guest.is_host() {
            return Err(LobbyError::CannotDelegateToNonGuest);
        }

        self.participants.insert(guest.id(), guest);
        Ok(())
    }

    /// Remove a participant by ID
    pub fn remove_participant(&mut self, participant_id: Uuid) -> Result<(), LobbyError> {
        if participant_id == self.host_id {
            return Err(LobbyError::NoHost);
        }

        self.participants
            .remove(&participant_id)
            .ok_or(LobbyError::ParticipantNotFound(participant_id))?;

        Ok(())
    }

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

        // Demote old host
        if let Some(old_host) = self.participants.get_mut(&self.host_id) {
            old_host.demote_to_guest();
        }

        // Update host ID
        self.host_id = new_host_id;

        Ok(())
    }

    /// Automatically delegate host to the oldest guest (deterministic election)
    pub fn auto_delegate_host(&mut self) -> Result<Uuid, LobbyError> {
        // Find oldest guest (earliest join timestamp)
        let oldest_guest = self
            .participants
            .values()
            .filter(|p| !p.is_host())
            .min_by_key(|p| p.joined_at())
            .ok_or(LobbyError::EmptyLobby)?;

        let new_host_id = oldest_guest.id();

        self.delegate_host(new_host_id)?;

        Ok(new_host_id)
    }

    /// Check if there are any guests in the lobby
    pub fn has_guests(&self) -> bool {
        self.participants.values().any(|p| !p.is_host())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::LobbyRole;

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

        assert_eq!(result, Err(LobbyError::NoHost));
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
}
