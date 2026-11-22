//! Local persistence of a session so a reload can reopen it.
//!
//! The library serializes a versioned [`SessionState`]; the application
//! supplies a [`SessionStore`] and decides where and how long it is kept
//! (browser `localStorage`, a file, memory in tests). Secrets are never part
//! of the state: the application keeps its key separately.

use crate::domain::SignedHistoryBatch;
use konnekt_session_core::{ActivityRun, Lobby};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Current format of [`SessionState`].
pub const SESSION_STATE_VERSION: u8 = 1;

/// Everything needed to reopen a lobby after a reload.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionState {
    pub version: u8,
    pub lobby: Lobby,
    /// Most recent runs with their results, oldest first.
    pub runs: Vec<ActivityRun>,
    /// The retained signed history and its checkpoint, if recorded.
    pub signed_history: Option<SignedHistoryBatch>,
}

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("stored session uses unsupported format version {0}")]
    UnsupportedVersion(u8),
    #[error("stored session cannot be read: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error("stored session belongs to another lobby")]
    WrongLobby,
    #[error("stored session is inconsistent: {0}")]
    Inconsistent(String),
    #[error("this client cannot host the stored lobby: it is not one of its participants")]
    NotAParticipant,
    #[error("stored signed history is invalid: {0}")]
    InvalidHistory(crate::application::signed_history::SignedHistoryError),
    #[error("only a host restores a stored session; guests synchronize from it")]
    NotHost,
    #[error("restore the session before the lobby exists")]
    LobbyExists,
}

impl SessionState {
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, PersistenceError> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let version = value
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .and_then(|version| u8::try_from(version).ok())
            .unwrap_or(0);
        if version != SESSION_STATE_VERSION {
            return Err(PersistenceError::UnsupportedVersion(version));
        }
        Ok(serde_json::from_value(value)?)
    }
}

/// Where a session is stored, keyed by lobby ID.
pub trait SessionStore: Send + Sync {
    fn load(&self, lobby_id: Uuid) -> Option<String>;
    fn save(&mut self, lobby_id: Uuid, state: &str);
    fn remove(&mut self, lobby_id: Uuid);
}

/// In-memory store; clones share their contents (useful for reload tests).
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    entries: Arc<Mutex<HashMap<Uuid, String>>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// The stored session of a lobby, if any.
    pub fn state(&self, lobby_id: Uuid) -> Option<Result<SessionState, PersistenceError>> {
        self.load(lobby_id)
            .map(|json| SessionState::from_json(&json))
    }
}

impl SessionStore for MemoryStore {
    fn load(&self, lobby_id: Uuid) -> Option<String> {
        self.entries.lock().ok()?.get(&lobby_id).cloned()
    }

    fn save(&mut self, lobby_id: Uuid, state: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.insert(lobby_id, state.to_owned());
        }
    }

    fn remove(&mut self, lobby_id: Uuid) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(&lobby_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_core::Participant;

    #[test]
    fn states_round_trip_and_unknown_versions_are_rejected() {
        let host = Participant::new_host("Host".into()).unwrap_or_else(|e| panic!("{e}"));
        let lobby = Lobby::new("Room".into(), host).unwrap_or_else(|e| panic!("{e}"));
        let state = SessionState {
            version: SESSION_STATE_VERSION,
            lobby: lobby.clone(),
            runs: Vec::new(),
            signed_history: None,
        };
        let mut store = MemoryStore::new();
        let reader = store.clone();
        store.save(
            lobby.id(),
            &state.to_json().unwrap_or_else(|e| panic!("{e}")),
        );
        let loaded = reader
            .state(lobby.id())
            .and_then(Result::ok)
            .unwrap_or_else(|| panic!("nothing stored"));
        assert_eq!(loaded.lobby, state.lobby);
        assert_eq!(loaded.version, SESSION_STATE_VERSION);

        let mut future = serde_json::to_value(&state).unwrap_or_else(|e| panic!("{e}"));
        future["version"] = 9.into();
        assert!(matches!(
            SessionState::from_json(&future.to_string()),
            Err(PersistenceError::UnsupportedVersion(9))
        ));
        assert!(matches!(
            SessionState::from_json("{\"version\":1}"),
            Err(PersistenceError::Malformed(_))
        ));
        store.remove(lobby.id());
        assert!(reader.state(lobby.id()).is_none());
    }
}
