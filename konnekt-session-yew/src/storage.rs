//! Browser `localStorage` store for reopening a session after a reload.

use gloo::storage::{LocalStorage, Storage};
use konnekt_session_p2p::application::persistence::SessionStore;
use uuid::Uuid;

/// Keeps one entry per lobby under `<prefix><lobby id>` in `localStorage`.
/// Writes that fail (e.g. quota exceeded, private mode) are logged and
/// skipped; the session keeps running.
#[derive(Debug, Clone)]
pub struct LocalStorageStore {
    prefix: String,
}

impl LocalStorageStore {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    fn key(&self, lobby_id: Uuid) -> String {
        format!("{}{lobby_id}", self.prefix)
    }
}

impl Default for LocalStorageStore {
    fn default() -> Self {
        Self::new("konnekt-session/lobby/")
    }
}

impl SessionStore for LocalStorageStore {
    fn load(&self, lobby_id: Uuid) -> Option<String> {
        LocalStorage::raw()
            .get_item(&self.key(lobby_id))
            .ok()
            .flatten()
    }

    fn save(&mut self, lobby_id: Uuid, state: &str) {
        if LocalStorage::raw()
            .set_item(&self.key(lobby_id), state)
            .is_err()
        {
            tracing::warn!(%lobby_id, "Unable to store the session in localStorage");
        }
    }

    fn remove(&mut self, lobby_id: Uuid) {
        let _ = LocalStorage::raw().remove_item(&self.key(lobby_id));
    }
}
