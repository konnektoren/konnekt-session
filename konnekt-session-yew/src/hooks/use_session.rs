use konnekt_session_core::{DomainCommand, Lobby};
use konnekt_session_p2p::SessionId;
use std::rc::Rc;
use yew::prelude::*;

/// Session state accessible via hook
#[derive(Clone)]
pub struct SessionContext {
    pub session_id: SessionId,
    pub lobby: Option<Lobby>,
    pub peer_count: usize,
    pub is_host: bool,

    /// Send commands to SessionLoop
    pub send_command: Rc<dyn Fn(DomainCommand)>,

    /// Get local participant ID
    pub local_participant_id: Option<uuid::Uuid>,
}

impl PartialEq for SessionContext {
    fn eq(&self, other: &Self) -> bool {
        self.session_id == other.session_id
            && self.lobby == other.lobby
            && self.peer_count == other.peer_count
            && self.is_host == other.is_host
            && self.local_participant_id == other.local_participant_id
    }
}

/// Hook to access session state
///
/// # Example
///
/// ```rust,no_run
/// use konnekt_session_yew::use_session;
/// use konnekt_session_core::DomainCommand;
///
/// let session = use_session();
///
/// // Send a command
/// (session.send_command)(DomainCommand::JoinLobby {
///     lobby_id: session.lobby.unwrap().id(),
///     guest_name: "Alice".to_string(),
/// });
/// ```
#[hook]
pub fn use_session() -> SessionContext {
    use_context::<SessionContext>().expect("use_session must be used within a SessionProvider")
}
