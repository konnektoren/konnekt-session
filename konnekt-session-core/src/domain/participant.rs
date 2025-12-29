use instant::Instant;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Role within the lobby - determines authority and permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Role determining participant permissions and capabilities")]
pub enum LobbyRole {
    /// Can manage lobby, kick guests, start activities, delegate role
    #[schemars(description = "Host role with full management privileges")]
    Host,
    /// Regular participant without management privileges
    #[schemars(description = "Guest role with standard participant capabilities")]
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

/// Participation mode - determines whether participant can play activities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Mode determining if participant can actively play or only watch")]
pub enum ParticipationMode {
    /// Can participate in activities and submit results
    #[schemars(description = "Actively participating - can submit activity results")]
    Active,
    /// View-only, cannot submit results
    #[schemars(description = "Spectating only - cannot submit results")]
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

impl Default for ParticipationMode {
    fn default() -> Self {
        // New guests join in Active mode by default (from requirements)
        ParticipationMode::Active
    }
}

/// Timestamp in milliseconds since application start (monotonic)
///
/// This is serializable and comparable, suitable for deterministic ordering.
/// Uses instant::Instant internally for WASM compatibility (ADR-0013).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[schemars(description = "Monotonic timestamp in milliseconds since session start")]
pub struct Timestamp(
    #[schemars(
        description = "Milliseconds since session start",
        example = "example_timestamp()"
    )]
    u64,
);

fn example_timestamp() -> u64 {
    12345
}

impl Timestamp {
    /// Create a timestamp representing the current moment
    pub fn now() -> Self {
        // Use a static anchor point for all timestamps in the session
        static ANCHOR: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let anchor = ANCHOR.get_or_init(Instant::now);

        let elapsed = Instant::now().duration_since(*anchor);
        Timestamp(elapsed.as_millis() as u64)
    }

    /// Get the raw milliseconds value
    pub fn as_millis(&self) -> u64 {
        self.0
    }

    /// Create a timestamp from a raw milliseconds value (for testing)
    #[cfg(test)]
    pub fn from_millis(millis: u64) -> Self {
        Timestamp(millis)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ms", self.0)
    }
}

/// Domain entity representing a participant in the lobby.
///
/// A participant can be either a Host (with management privileges) or a Guest (regular participant).
/// Participants can also switch between Active (can play) and Spectating (view-only) modes.
///
/// # Examples
///
/// ```json
/// {
///   "id": "550e8400-e29b-41d4-a716-446655440000",
///   "name": "Alice",
///   "lobby_role": "Host",
///   "participation_mode": "Active",
///   "joined_at": 12345
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "A participant in a lobby session")]
pub struct Participant {
    /// Unique identifier for this participant
    #[schemars(
        description = "Unique participant identifier (UUID v4)",
        example = "example_uuid()"
    )]
    id: Uuid,

    /// Display name (unique within lobby)
    #[schemars(
        description = "Display name (must be unique within lobby)",
        length(min = 1, max = 50),
        example = "example_name()"
    )]
    name: String,

    /// Role determining permissions
    #[schemars(description = "Participant's role (Host or Guest)")]
    lobby_role: LobbyRole,

    /// Participation mode determining activity involvement
    #[schemars(description = "Participation mode (Active or Spectating)")]
    participation_mode: ParticipationMode,

    /// Timestamp when participant joined (for host election)
    /// Monotonic timestamp in milliseconds (WASM-compatible via ADR-0013)
    #[schemars(
        description = "Monotonic timestamp when participant joined (for deterministic host election)"
    )]
    joined_at: Timestamp,
}

fn example_uuid() -> &'static str {
    "550e8400-e29b-41d4-a716-446655440000"
}

fn example_name() -> &'static str {
    "Alice"
}

/// Errors that can occur when working with participants
#[derive(Debug, thiserror::Error, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Participant validation and operation errors")]
pub enum ParticipantError {
    #[error("Name cannot be empty")]
    #[schemars(description = "Attempted to create participant with empty name")]
    EmptyName,

    #[error("Name must be between 1 and 50 characters")]
    #[schemars(description = "Name length exceeds maximum (50 characters)")]
    InvalidNameLength,

    #[error("Cannot change participation mode during active activity")]
    #[schemars(
        description = "Attempted to toggle participation mode while activity is in progress"
    )]
    CannotToggleDuringActivity,
}

// ... rest of implementation stays the same ...

impl Participant {
    /// Create a new participant with Host role
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

    /// Create a new participant with Guest role
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

    /// Create a participant with an explicit timestamp (for testing or deserialization)
    #[cfg(test)]
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

    /// Validate name according to business rules
    fn validate_name(name: &str) -> Result<(), ParticipantError> {
        if name.is_empty() {
            return Err(ParticipantError::EmptyName);
        }

        if name.len() > 50 {
            return Err(ParticipantError::InvalidNameLength);
        }

        Ok(())
    }

    // Getters

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

    // Business logic queries

    /// Check if this participant is the host
    pub fn is_host(&self) -> bool {
        matches!(self.lobby_role, LobbyRole::Host)
    }

    /// Check if this participant can submit activity results
    pub fn can_submit_results(&self) -> bool {
        matches!(self.participation_mode, ParticipationMode::Active)
    }

    /// Check if this participant can manage the lobby
    pub fn can_manage_lobby(&self) -> bool {
        self.is_host()
    }

    // State mutations

    /// Toggle participation mode (Active ↔ Spectating)
    /// Returns error if activity is currently running
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

    /// Force set participation mode (used by host)
    pub fn force_participation_mode(&mut self, mode: ParticipationMode) {
        self.participation_mode = mode;
    }

    /// Promote this participant to host role
    pub fn promote_to_host(&mut self) {
        self.lobby_role = LobbyRole::Host;
    }

    /// Demote this participant to guest role
    pub fn demote_to_guest(&mut self) {
        self.lobby_role = LobbyRole::Guest;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use instant::Duration;
    use schemars::schema_for;

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

    #[test]
    fn test_participant_json_schema() {
        let schema = schema_for!(Participant);

        // Convert to JSON and verify it's valid
        let json = serde_json::to_string(&schema).unwrap();
        assert!(!json.is_empty());

        // Verify schema contains expected $schema field
        let schema_json: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(schema_json.get("$schema").is_some());
    }

    #[test]
    fn test_lobby_role_json_schema() {
        let schema = schema_for!(LobbyRole);

        // Verify we can generate a schema for enum
        let json = serde_json::to_string(&schema).unwrap();
        assert!(!json.is_empty());

        // Should be a valid JSON schema
        let schema_json: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(schema_json.get("$schema").is_some());
    }

    #[test]
    fn test_participation_mode_json_schema() {
        let schema = schema_for!(ParticipationMode);

        // Verify we can generate a schema for enum
        let json = serde_json::to_string(&schema).unwrap();
        assert!(!json.is_empty());

        // Should be a valid JSON schema
        let schema_json: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(schema_json.get("$schema").is_some());
    }

    #[test]
    fn test_participant_error_json_schema() {
        let schema = schema_for!(ParticipantError);

        // Verify we can generate a schema for error enum
        let json = serde_json::to_string(&schema).unwrap();
        assert!(!json.is_empty());
    }

    #[test]
    fn test_timestamp_json_schema() {
        let schema = schema_for!(Timestamp);

        // Verify we can generate a schema
        let json = serde_json::to_string(&schema).unwrap();
        assert!(!json.is_empty());
    }
}
