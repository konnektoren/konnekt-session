use crate::hooks::SessionContext;
use futures::StreamExt;
use konnekt_session_core::{DomainCommand, Lobby};
use konnekt_session_p2p::{IceServer, P2PLoopBuilder, SessionId};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SessionProviderProps {
    /// Matchbox signalling server URL
    pub signalling_server: AttrValue,

    /// Session ID (for joining existing session)
    #[prop_or_default]
    pub session_id: Option<AttrValue>,

    /// Display name for this user
    #[prop_or_default]
    pub name: Option<AttrValue>,

    /// Children components
    pub children: Children,
}

/// Provides session state to child components
///
/// # Example
///
/// ```rust,no_run
/// use yew::prelude::*;
/// use konnekt_session_yew::SessionProvider;
///
/// #[function_component(App)]
/// fn app() -> Html {
///     html! {
///         <SessionProvider signalling_server="wss://match.konnektoren.help">
///             // Your components here
///         </SessionProvider>
///     }
/// }
/// ```
#[function_component(SessionProvider)]
pub fn session_provider(props: &SessionProviderProps) -> Html {
    let lobby = use_state(|| None::<Lobby>);
    let peer_count = use_state(|| 0usize);
    let is_host = use_state(|| false);
    let actual_session_id = use_state(|| SessionId::new());

    // Initialize session on mount
    {
        let signalling_server = props.signalling_server.to_string();
        let session_id_prop = props.session_id.clone();
        let name = props.name.clone().unwrap_or_else(|| "Guest".into());
        let is_host = is_host.clone();
        let actual_session_id = actual_session_id.clone();
        let lobby = lobby.clone();
        let peer_count = peer_count.clone();

        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let ice_servers = IceServer::default_stun_servers();

                let (mut session_loop, sid) = if let Some(sid_str) = session_id_prop {
                    // Join existing session
                    let sid = SessionId::parse(&sid_str).expect("Invalid session ID");
                    let (mut loop_, lobby_id) = P2PLoopBuilder::new()
                        .build_session_guest(&signalling_server, sid.clone(), ice_servers)
                        .await
                        .expect("Failed to join session");

                    // Wait for sync
                    for _ in 0..100 {
                        loop_.poll();
                        if loop_.get_lobby().is_some() {
                            break;
                        }
                        gloo_timers::future::TimeoutFuture::new(100).await;
                    }

                    // Submit join command
                    loop_
                        .submit_command(DomainCommand::JoinLobby {
                            lobby_id,
                            guest_name: name.to_string(),
                        })
                        .expect("Failed to submit join command");

                    is_host.set(false);
                    (loop_, sid)
                } else {
                    // Create new session as host
                    let (loop_, sid) = P2PLoopBuilder::new()
                        .build_session_host(
                            &signalling_server,
                            ice_servers,
                            "Yew Lobby".to_string(),
                            name.to_string(),
                        )
                        .await
                        .expect("Failed to create session");

                    is_host.set(true);
                    (loop_, sid)
                };

                actual_session_id.set(sid);

                // 🔧 FIX: Use consistent return type
                let mut interval = gloo_timers::future::IntervalStream::new(100);

                while interval.next().await.is_some() {
                    session_loop.poll();

                    if let Some(l) = session_loop.get_lobby() {
                        lobby.set(Some(l.clone()));
                    }

                    peer_count.set(session_loop.connected_peers().len());
                }
            });

            // 🔧 FIX: Return a cleanup function (even if it does nothing)
            move || {}
        });
    }

    let context = SessionContext {
        session_id: (*actual_session_id).clone(),
        lobby: (*lobby).clone(),
        peer_count: *peer_count,
        is_host: *is_host,
    };

    html! {
        <ContextProvider<SessionContext> {context}>
            {props.children.clone()}
        </ContextProvider<SessionContext>>
    }
}
