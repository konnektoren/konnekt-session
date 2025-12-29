use crate::infrastructure::error::{P2PError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Domain entity: Unique identifier for a P2P session (lobby)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);

impl SessionId {
    /// Create a new random session ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse a session ID from a string
    pub fn parse(s: &str) -> Result<Self> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|e| P2PError::InvalidSessionId(e.to_string()))
    }

    /// Get the session ID as a string
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }

    pub fn inner(&self) -> Uuid {
        self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_new() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_session_id_parse() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let session_id = SessionId::parse(uuid_str).unwrap();

        assert_eq!(session_id.as_str(), uuid_str);
    }

    #[test]
    fn test_session_id_parse_invalid() {
        let result = SessionId::parse("not-a-uuid");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_id_display() {
        let session_id = SessionId::new();
        let display = session_id.to_string();

        // Should be a valid UUID string
        assert!(SessionId::parse(&display).is_ok());
    }

    #[test]
    fn test_session_id_serialization() {
        let session_id = SessionId::new();

        let json = serde_json::to_string(&session_id).unwrap();
        let deserialized: SessionId = serde_json::from_str(&json).unwrap();

        assert_eq!(session_id, deserialized);
    }

    #[test]
    fn test_session_id_default() {
        let id1 = SessionId::default();
        let id2 = SessionId::default();

        // Default should create new UUIDs
        assert_ne!(id1, id2);
    }
}
