use konnekt_session_core::Lobby;
use konnekt_session_p2p::SessionId;
use yew::prelude::*;

/// Session state accessible via hook
#[derive(Clone, PartialEq)]
pub struct SessionContext {
    pub session_id: SessionId,
    pub lobby: Option<Lobby>,
    pub peer_count: usize,
    pub is_host: bool,
}

/// Hook to access session state
///
/// # Example
///
/// ```rust,no_run
/// use konnekt_session_yew::use_session;
///
/// let session = use_session();
/// if let Some(lobby) = session.lobby.as_ref() {
///     html! { <p>{ format!("Lobby: {}", lobby.name()) }</p> }
/// }
/// ```
#[hook]
pub fn use_session() -> SessionContext {
    use_context::<SessionContext>().expect("use_session must be used within a SessionProvider")
}
