use instant::Instant;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LobbyRole {
    Host,
    Guest,
}

impl fmt::Display for LobbyRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LobbyRole::Host => write!(f, "Host"),
            LobbyRole::Guest => write!(f, "Guest"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ParticipationMode {
    #[default]
    Active,
    Spectating,
}

impl fmt::Display for ParticipationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParticipationMode::Active => write!(f, "Active"),
            ParticipationMode::Spectating => write!(f, "Spectating"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(u64);

impl Timestamp {
    pub fn now() -> Self {
        static ANCHOR: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let anchor = ANCHOR.get_or_init(Instant::now);
        let elapsed = Instant::now().duration_since(*anchor);
        Timestamp(elapsed.as_millis() as u64)
    }

    pub fn as_millis(&self) -> u64 {
        self.0
    }

    pub fn from_millis(millis: u64) -> Self {
        Timestamp(millis)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ms", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participant {
    id: Uuid,
    name: String,
    lobby_role: LobbyRole,
    participation_mode: ParticipationMode,
    joined_at: Timestamp,
}

#[derive(Debug, thiserror::Error, PartialEq, Serialize, Deserialize)]
pub enum ParticipantError {
    #[error("Name cannot be empty")]
    EmptyName,

    #[error("Name must be between 1 and 50 characters")]
    InvalidNameLength,

    #[error("Cannot change participation mode during active activity")]
    CannotToggleDuringActivity,
}

impl Participant {
    pub fn new_host(name: String) -> Result<Self, ParticipantError> {
        Self::validate_name(&name)?;
        Ok(Participant {
            id: Uuid::new_v4(),
            name,
            lobby_role: LobbyRole::Host,
            participation_mode: ParticipationMode::Active,
            joined_at: Timestamp::now(),
        })
    }

    pub fn new_guest(name: String) -> Result<Self, ParticipantError> {
        Self::validate_name(&name)?;
        Ok(Participant {
            id: Uuid::new_v4(),
            name,
            lobby_role: LobbyRole::Guest,
            participation_mode: ParticipationMode::default(),
            joined_at: Timestamp::now(),
        })
    }

    pub fn with_id(
        id: Uuid,
        name: String,
        lobby_role: LobbyRole,
        participation_mode: ParticipationMode,
        joined_at: Timestamp,
    ) -> Result<Self, ParticipantError> {
        Self::validate_name(&name)?;
        Ok(Participant {
            id,
            name,
            lobby_role,
            participation_mode,
            joined_at,
        })
    }

    pub fn host_with_id(id: Uuid, name: String) -> Result<Self, ParticipantError> {
        Self::with_id(
            id,
            name,
            LobbyRole::Host,
            ParticipationMode::Active,
            Timestamp::now(),
        )
    }

    pub fn guest_with_id(id: Uuid, name: String) -> Result<Self, ParticipantError> {
        Self::with_id(
            id,
            name,
            LobbyRole::Guest,
            ParticipationMode::default(),
            Timestamp::now(),
        )
    }

    pub fn with_timestamp(
        name: String,
        lobby_role: LobbyRole,
        joined_at: Timestamp,
    ) -> Result<Self, ParticipantError> {
        Self::validate_name(&name)?;
        Ok(Participant {
            id: Uuid::new_v4(),
            name,
            lobby_role,
            participation_mode: ParticipationMode::default(),
            joined_at,
        })
    }

    fn validate_name(name: &str) -> Result<(), ParticipantError> {
        if name.is_empty() {
            return Err(ParticipantError::EmptyName);
        }
        if name.len() > 50 {
            return Err(ParticipantError::InvalidNameLength);
        }
        Ok(())
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn lobby_role(&self) -> LobbyRole {
        self.lobby_role
    }

    pub fn participation_mode(&self) -> ParticipationMode {
        self.participation_mode
    }

    pub fn joined_at(&self) -> Timestamp {
        self.joined_at
    }

    pub fn is_host(&self) -> bool {
        matches!(self.lobby_role, LobbyRole::Host)
    }

    pub fn can_submit_results(&self) -> bool {
        matches!(self.participation_mode, ParticipationMode::Active)
    }

    pub fn can_manage_lobby(&self) -> bool {
        self.is_host()
    }

    pub fn toggle_participation_mode(
        &mut self,
        activity_in_progress: bool,
    ) -> Result<ParticipationMode, ParticipantError> {
        if activity_in_progress {
            return Err(ParticipantError::CannotToggleDuringActivity);
        }

        self.participation_mode = match self.participation_mode {
            ParticipationMode::Active => ParticipationMode::Spectating,
            ParticipationMode::Spectating => ParticipationMode::Active,
        };

        Ok(self.participation_mode)
    }

    pub fn force_participation_mode(&mut self, mode: ParticipationMode) {
        self.participation_mode = mode;
    }

    pub fn promote_to_host(&mut self) {
        self.lobby_role = LobbyRole::Host;
    }

    pub fn demote_to_guest(&mut self) {
        self.lobby_role = LobbyRole::Guest;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use instant::Duration;

    #[test]
    fn test_create_host() {
        let host = Participant::new_host("Alice".to_string()).unwrap();

        assert_eq!(host.name(), "Alice");
        assert_eq!(host.lobby_role(), LobbyRole::Host);
        assert_eq!(host.participation_mode(), ParticipationMode::Active);
        assert!(host.is_host());
        assert!(host.can_manage_lobby());
        assert!(host.can_submit_results());
    }

    #[test]
    fn test_create_guest() {
        let guest = Participant::new_guest("Bob".to_string()).unwrap();

        assert_eq!(guest.name(), "Bob");
        assert_eq!(guest.lobby_role(), LobbyRole::Guest);
        assert_eq!(guest.participation_mode(), ParticipationMode::Active);
        assert!(!guest.is_host());
        assert!(!guest.can_manage_lobby());
        assert!(guest.can_submit_results());
    }

    #[test]
    fn test_empty_name_validation() {
        let result = Participant::new_guest("".to_string());

        assert_eq!(result, Err(ParticipantError::EmptyName));
    }

    #[test]
    fn test_name_length_validation() {
        let long_name = "a".repeat(51);
        let result = Participant::new_guest(long_name);

        assert_eq!(result, Err(ParticipantError::InvalidNameLength));
    }

    #[test]
    fn test_toggle_participation_mode_when_no_activity() {
        let mut guest = Participant::new_guest("Carol".to_string()).unwrap();
        assert_eq!(guest.participation_mode(), ParticipationMode::Active);

        let result = guest.toggle_participation_mode(false);
        assert!(result.is_ok());
        assert_eq!(guest.participation_mode(), ParticipationMode::Spectating);
        assert!(!guest.can_submit_results());

        let result = guest.toggle_participation_mode(false);
        assert!(result.is_ok());
        assert_eq!(guest.participation_mode(), ParticipationMode::Active);
        assert!(guest.can_submit_results());
    }

    #[test]
    fn test_cannot_toggle_during_activity() {
        let mut guest = Participant::new_guest("Carol".to_string()).unwrap();

        let result = guest.toggle_participation_mode(true);

        assert_eq!(result, Err(ParticipantError::CannotToggleDuringActivity));
        assert_eq!(guest.participation_mode(), ParticipationMode::Active);
    }

    #[test]
    fn test_force_participation_mode() {
        let mut guest = Participant::new_guest("Dave".to_string()).unwrap();
        assert_eq!(guest.participation_mode(), ParticipationMode::Active);

        guest.force_participation_mode(ParticipationMode::Spectating);

        assert_eq!(guest.participation_mode(), ParticipationMode::Spectating);
        assert!(!guest.can_submit_results());
    }

    #[test]
    fn test_host_can_be_spectating() {
        let mut host = Participant::new_host("Alice".to_string()).unwrap();

        host.toggle_participation_mode(false).unwrap();

        assert_eq!(host.participation_mode(), ParticipationMode::Spectating);
        assert!(!host.can_submit_results());
        assert!(host.can_manage_lobby());
    }

    #[test]
    fn test_promote_to_host() {
        let mut guest = Participant::new_guest("Bob".to_string()).unwrap();
        assert!(!guest.is_host());

        guest.promote_to_host();

        assert!(guest.is_host());
        assert!(guest.can_manage_lobby());
        assert_eq!(guest.lobby_role(), LobbyRole::Host);
    }

    #[test]
    fn test_demote_to_guest() {
        let mut host = Participant::new_host("Alice".to_string()).unwrap();
        assert!(host.is_host());

        host.demote_to_guest();

        assert!(!host.is_host());
        assert!(!host.can_manage_lobby());
        assert_eq!(host.lobby_role(), LobbyRole::Guest);
    }

    #[test]
    fn test_joined_at_timestamp_ordering() {
        let guest1 = Participant::new_guest("Alice".to_string()).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        let guest2 = Participant::new_guest("Bob".to_string()).unwrap();

        assert!(guest2.joined_at() > guest1.joined_at());
    }

    #[test]
    fn test_unique_ids() {
        let guest1 = Participant::new_guest("Alice".to_string()).unwrap();
        let guest2 = Participant::new_guest("Alice".to_string()).unwrap();

        assert_ne!(guest1.id(), guest2.id());
    }

    #[test]
    fn test_display_lobby_role() {
        assert_eq!(LobbyRole::Host.to_string(), "Host");
        assert_eq!(LobbyRole::Guest.to_string(), "Guest");
    }

    #[test]
    fn test_display_participation_mode() {
        assert_eq!(ParticipationMode::Active.to_string(), "Active");
        assert_eq!(ParticipationMode::Spectating.to_string(), "Spectating");
    }

    #[test]
    fn test_participation_mode_default() {
        assert_eq!(ParticipationMode::default(), ParticipationMode::Active);
    }

    #[test]
    fn test_timestamp_ordering() {
        let t1 = Timestamp::from_millis(100);
        let t2 = Timestamp::from_millis(200);
        let t3 = Timestamp::from_millis(200);

        assert!(t1 < t2);
        assert!(t2 > t1);
        assert_eq!(t2, t3);
    }

    #[test]
    fn test_timestamp_serialization() {
        let timestamp = Timestamp::from_millis(12345);
        let json = serde_json::to_string(&timestamp).unwrap();
        assert_eq!(json, "12345");

        let deserialized: Timestamp = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, timestamp);
    }

    #[test]
    fn test_participant_serialization() {
        let participant = Participant::with_timestamp(
            "Alice".to_string(),
            LobbyRole::Host,
            Timestamp::from_millis(1000),
        )
        .unwrap();

        let json = serde_json::to_string(&participant).unwrap();
        let deserialized: Participant = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name(), participant.name());
        assert_eq!(deserialized.lobby_role(), participant.lobby_role());
        assert_eq!(deserialized.joined_at(), participant.joined_at());
    }

    #[test]
    fn test_timestamp_display() {
        let timestamp = Timestamp::from_millis(12345);
        assert_eq!(timestamp.to_string(), "12345ms");
    }

    #[test]
    fn test_timestamp_now_is_monotonic() {
        let t1 = Timestamp::now();
        std::thread::sleep(Duration::from_millis(5));
        let t2 = Timestamp::now();

        assert!(t2 > t1);
    }

    #[test]
    fn test_deterministic_election_by_timestamp() {
        let alice = Participant::with_timestamp(
            "Alice".to_string(),
            LobbyRole::Guest,
            Timestamp::from_millis(100),
        )
        .unwrap();

        let bob = Participant::with_timestamp(
            "Bob".to_string(),
            LobbyRole::Guest,
            Timestamp::from_millis(200),
        )
        .unwrap();

        let carol = Participant::with_timestamp(
            "Carol".to_string(),
            LobbyRole::Guest,
            Timestamp::from_millis(150),
        )
        .unwrap();

        let mut participants = vec![bob.clone(), carol.clone(), alice.clone()];
        participants.sort_by_key(|p| p.joined_at());

        assert_eq!(participants[0].name(), "Alice");
        assert_eq!(participants[1].name(), "Carol");
        assert_eq!(participants[2].name(), "Bob");
    }
}
