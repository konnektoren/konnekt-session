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
/// ```rust,ignore
/// use konnekt_session_yew::use_session;
///
/// #[function_component]
/// fn MyComponent() -> Html {
///     let session = use_session();
///
///     // Access session properties
///     let session_id = &session.session_id;
///     let is_host = session.is_host;
///     let peer_count = session.peer_count;
///
///     html! {
///         <div>
///             <p>{format!("Peers: {}", peer_count)}</p>
///         </div>
///     }
/// }
/// ```
#[hook]
pub fn use_session() -> SessionContext {
    use_context::<SessionContext>().expect("use_session must be used within a SessionProvider")
}
