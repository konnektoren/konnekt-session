use konnekt_session_core::Lobby;
use yew::prelude::*;

use super::use_session;

/// Hook to access lobby state (convenience wrapper)
///
/// Returns `None` if lobby hasn't synced yet.
#[hook]
pub fn use_lobby() -> Option<Lobby> {
    let session = use_session();
    session.lobby
}
