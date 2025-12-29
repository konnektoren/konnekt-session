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

    #[error("Cannot remove host without delegation")]
    CannotRemoveHost,

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

    /// Get mutable access to participants (for removing timed out peers)
    pub fn participants_mut(&mut self) -> &mut HashMap<Uuid, Participant> {
        &mut self.participants
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
        // Reproduce the bug: host is removed from participants map,
        // then auto_delegate tries to find them

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
        // This is what happens in handle_peer_timed_out
        lobby.participants_mut().remove(&host_id);

        // Now try to auto-delegate
        // This should work - promote Alice (oldest guest)
        let result = lobby.auto_delegate_host();

        assert!(
            result.is_ok(),
            "auto_delegate should succeed even if host is removed"
        );
        let new_host_id = result.unwrap();
        assert_eq!(new_host_id, alice_id, "Alice should become host");

        // Verify Alice is now host
        assert!(lobby.participants().get(&alice_id).unwrap().is_host());
        assert_eq!(lobby.host_id(), alice_id);
    }

    #[test]
    fn test_delegate_host_with_removed_old_host() {
        // Test that delegate_host works when old host is already removed

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

        // This SHOULD succeed - we handle missing old host gracefully
        assert!(
            result.is_ok(),
            "Should succeed even when old host is removed"
        );
        assert_eq!(lobby.host_id(), alice_id);
        assert!(lobby.participants().get(&alice_id).unwrap().is_host());
    }

    #[test]
    fn test_lobby_state_after_host_timeout() {
        // Reproduce the exact scenario from the error

        let host = Participant::new_host("OriginalHost".to_string()).unwrap();
        let host_id = host.id();

        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let guest1 = Participant::new_guest("Guest1".to_string()).unwrap();
        let guest1_id = guest1.id();

        let guest2 = Participant::new_guest("Guest2".to_string()).unwrap();

        lobby.add_guest(guest1).unwrap();
        lobby.add_guest(guest2).unwrap();

        println!("Before timeout:");
        println!("  Host ID: {}", lobby.host_id());
        println!("  Participants: {}", lobby.participants().len());
        println!(
            "  Host exists in map: {}",
            lobby.participants().contains_key(&host_id)
        );

        // Simulate host timeout:
        // 1. Remove host from participants
        let removed = lobby.participants_mut().remove(&host_id);
        assert!(removed.is_some(), "Host should be in participants");

        println!("\nAfter removing host:");
        println!("  Host ID (still): {}", lobby.host_id());
        println!("  Participants: {}", lobby.participants().len());
        println!(
            "  Host exists in map: {}",
            lobby.participants().contains_key(&host_id)
        );

        // 2. Try to delegate
        let result = lobby.auto_delegate_host();

        println!("\nDelegation result: {:?}", result);

        if let Ok(new_host_id) = result {
            println!("New host ID: {}", new_host_id);
            println!("New host is Guest1: {}", new_host_id == guest1_id);

            // Verify state
            assert_eq!(lobby.host_id(), new_host_id);
            assert!(lobby.participants().get(&new_host_id).unwrap().is_host());
        } else {
            panic!("Delegation should succeed");
        }
    }
}
