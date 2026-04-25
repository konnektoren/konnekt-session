use konnekt_session_core::{
    DomainCommand, Lobby, LobbyRole, Participant, ParticipationMode, RunStatus,
};
use konnekt_session_p2p::SessionId;
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveRunSnapshot {
    pub run_id: Uuid,
    pub status: RunStatus,
    pub name: String,
    pub config: serde_json::Value,
    pub required_submitters: Vec<Uuid>,
    pub results: Vec<konnekt_session_core::domain::ActivityResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum P2PRole {
    Host,
    Guest,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhoAmI {
    pub local_peer_id: Option<String>,
    pub p2p_role: P2PRole,
    pub participant_id: Option<Uuid>,
    pub participant_name: Option<String>,
    pub lobby_role: Option<LobbyRole>,
    pub participation_mode: Option<ParticipationMode>,
}

/// Session state accessible via hook
#[derive(Clone)]
pub struct SessionContext {
    pub session_id: SessionId,
    pub lobby: Option<Lobby>,
    pub peer_count: usize,
    pub is_host: bool,
    pub active_run: Option<ActiveRunSnapshot>,
    pub local_participant_id: Option<Uuid>,
    pub local_peer_id: Option<String>,

    /// Send commands to SessionLoop
    pub send_command: Rc<dyn Fn(DomainCommand)>,

    /// Our participant name (immutable)
    pub local_participant_name: Option<String>,
}

impl SessionContext {
    /// Rich identity view combining P2P and lobby/domain identity.
    pub fn who_am_i_info(&self) -> WhoAmI {
        let participant = self.who_am_i();

        WhoAmI {
            local_peer_id: self.local_peer_id.clone(),
            p2p_role: if self.is_host {
                P2PRole::Host
            } else {
                P2PRole::Guest
            },
            participant_id: participant.map(|p| p.id()),
            participant_name: participant.map(|p| p.name().to_string()),
            lobby_role: participant.map(|p| p.lobby_role()),
            participation_mode: participant.map(|p| p.participation_mode()),
        }
    }

    /// Resolve local participant from current lobby state.
    ///
    /// Resolution order:
    /// 1. Runtime-resolved participant ID (peer registry mapping)
    /// 2. Name + role fallback (legacy path)
    pub fn who_am_i(&self) -> Option<&Participant> {
        let lobby = self.lobby.as_ref()?;

        if let Some(participant_id) = self.local_participant_id {
            if let Some(p) = lobby.participants().get(&participant_id) {
                return Some(p);
            }
        }

        let name = self.local_participant_name.as_ref()?;
        lobby
            .participants()
            .values()
            .find(|p| p.name() == name.as_str() && p.is_host() == self.is_host)
    }

    /// Get our participant from the lobby (single source of truth)
    pub fn get_local_participant(&self) -> Option<&Participant> {
        self.who_am_i()
    }

    /// Get our participant ID (looked up from core)
    pub fn get_local_participant_id(&self) -> Option<Uuid> {
        self.local_participant_id
            .or_else(|| self.get_local_participant().map(|p| p.id()))
    }
}

impl PartialEq for SessionContext {
    fn eq(&self, other: &Self) -> bool {
        self.session_id == other.session_id
            && self.lobby == other.lobby
            && self.peer_count == other.peer_count
            && self.is_host == other.is_host
            && self.active_run == other.active_run
            && self.local_participant_id == other.local_participant_id
            && self.local_peer_id == other.local_peer_id
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
