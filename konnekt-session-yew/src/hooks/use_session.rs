use konnekt_session_core::{DomainCommand, Lobby, Participant};
use konnekt_session_p2p::SessionId;
use std::rc::Rc;
use uuid::Uuid;
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

    /// Our participant name (immutable)
    pub local_participant_name: Option<String>,
}

impl SessionContext {
    /// Get our participant from the lobby (single source of truth)
    pub fn get_local_participant(&self) -> Option<&Participant> {
        let lobby = self.lobby.as_ref()?;
        let name = self.local_participant_name.as_ref()?;

        lobby
            .participants()
            .values()
            .find(|p| p.name() == name.as_str() && p.is_host() == self.is_host)
    }

    /// Get our participant ID (looked up from core)
    pub fn get_local_participant_id(&self) -> Option<Uuid> {
        self.get_local_participant().map(|p| p.id())
    }
}

impl PartialEq for SessionContext {
    fn eq(&self, other: &Self) -> bool {
        self.session_id == other.session_id
            && self.lobby == other.lobby
            && self.peer_count == other.peer_count
            && self.is_host == other.is_host
            && self.local_participant_name == other.local_participant_name
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
/// // Get our participant ID from core
/// if let Some(participant_id) = session.get_local_participant_id() {
///     // Send a command
///     (session.send_command)(DomainCommand::ToggleParticipationMode {
///         lobby_id: session.lobby.unwrap().id(),
///         participant_id,
///         requester_id: participant_id,
///         activity_in_progress: false,
///     });
/// }
/// ```
#[hook]
pub fn use_session() -> SessionContext {
    use_context::<SessionContext>().expect("use_session must be used within a SessionProvider")
}
