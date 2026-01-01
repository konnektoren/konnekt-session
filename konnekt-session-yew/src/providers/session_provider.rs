use crate::hooks::SessionContext;
use futures::StreamExt;
use konnekt_session_core::{DomainCommand, Lobby};
use konnekt_session_p2p::{IceServer, P2PLoopBuilder, SessionId};
use std::cell::RefCell;
use std::rc::Rc;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SessionProviderProps {
    pub signalling_server: AttrValue,
    #[prop_or_default]
    pub session_id: Option<AttrValue>,
    #[prop_or_default]
    pub name: Option<AttrValue>,
    pub children: Children,
}

struct SessionState {
    command_queue: Vec<DomainCommand>,
}

impl SessionState {
    fn new() -> Self {
        Self {
            command_queue: Vec::new(),
        }
    }

    fn enqueue_command(&mut self, cmd: DomainCommand) {
        self.command_queue.push(cmd);
    }

    fn drain_commands(&mut self) -> Vec<DomainCommand> {
        std::mem::take(&mut self.command_queue)
    }
}

#[function_component(SessionProvider)]
pub fn session_provider(props: &SessionProviderProps) -> Html {
    let lobby = use_state(|| None::<Lobby>);
    let peer_count = use_state(|| 0usize);
    let is_host = use_state(|| false);
    let actual_session_id = use_state(|| SessionId::new());
    let local_participant_name = use_state(|| None::<String>);

    let session_state = use_mut_ref(SessionState::new);

    let send_command = {
        let session_state = session_state.clone();
        Rc::new(move |cmd: DomainCommand| {
            session_state.borrow_mut().enqueue_command(cmd);
        }) as Rc<dyn Fn(DomainCommand)>
    };

    {
        let signalling_server = props.signalling_server.to_string();
        let session_id_prop = props.session_id.clone();
        let name = props.name.clone().unwrap_or_else(|| "Guest".into());
        let is_host_clone = is_host.clone();
        let actual_session_id_clone = actual_session_id.clone();
        let lobby_clone = lobby.clone();
        let peer_count_clone = peer_count.clone();
        let local_participant_name_clone = local_participant_name.clone();
        let session_state_clone = session_state.clone();

        use_effect_with((), move |_| {
            tracing::info!("🚀 SessionProvider starting");

            wasm_bindgen_futures::spawn_local(async move {
                let ice_servers = IceServer::default_stun_servers();

                let (mut session_loop, sid) = if let Some(sid_str) = session_id_prop {
                    let sid = SessionId::parse(&sid_str).expect("Invalid session ID");
                    tracing::info!("🔗 Joining session: {}", sid);

                    let (mut loop_, lobby_id) = P2PLoopBuilder::new()
                        .build_session_guest(&signalling_server, sid.clone(), ice_servers)
                        .await
                        .expect("Failed to join session");

                    // Wait for peer connection
                    tracing::info!("⏳ Waiting for peer connection...");
                    for i in 0..100 {
                        loop_.poll();
                        if !loop_.connected_peers().is_empty() {
                            tracing::info!(
                                "✅ Connected to {} peer(s)",
                                loop_.connected_peers().len()
                            );
                            break;
                        }

                        if i == 99 {
                            tracing::error!("❌ Timeout: No peer connection");
                        }

                        gloo_timers::future::TimeoutFuture::new(100).await;
                    }

                    // Wait for lobby sync
                    tracing::info!("⏳ Waiting for lobby sync...");
                    for i in 0..100 {
                        loop_.poll();
                        if loop_.get_lobby().is_some() {
                            tracing::info!("✅ Lobby synced");
                            break;
                        }

                        if i == 99 {
                            tracing::error!("❌ Timeout: Lobby never synced");
                        }

                        gloo_timers::future::TimeoutFuture::new(100).await;
                    }

                    // Submit join command
                    tracing::info!("📤 Submitting JoinLobby as '{}'", name);
                    if let Err(e) = loop_.submit_command(DomainCommand::JoinLobby {
                        lobby_id,
                        guest_name: name.to_string(),
                    }) {
                        tracing::error!("❌ Failed to join: {:?}", e);
                    }

                    // Wait to see ourselves in lobby
                    let name_str = name.to_string();
                    for i in 0..50 {
                        loop_.poll();

                        if let Some(lobby) = loop_.get_lobby() {
                            if lobby
                                .participants()
                                .values()
                                .any(|p| p.name() == name_str.as_str() && !p.is_host())
                            {
                                tracing::info!("✅ Found ourselves in lobby");
                                break;
                            }
                        }

                        if i == 49 {
                            tracing::error!("❌ Timeout: Never appeared in lobby");
                        }

                        gloo_timers::future::TimeoutFuture::new(100).await;
                    }

                    // Store our name
                    local_participant_name_clone.set(Some(name.to_string()));
                    is_host_clone.set(false);

                    (loop_, sid)
                } else {
                    // Host creation
                    tracing::info!("👑 Creating host session as '{}'", name);

                    let (loop_, sid) = P2PLoopBuilder::new()
                        .build_session_host(
                            &signalling_server,
                            ice_servers,
                            "Yew Lobby".to_string(),
                            name.to_string(),
                        )
                        .await
                        .expect("Failed to create session");

                    // Store our name
                    local_participant_name_clone.set(Some(name.to_string()));
                    is_host_clone.set(true);

                    (loop_, sid)
                };

                actual_session_id_clone.set(sid);

                let mut interval = gloo_timers::future::IntervalStream::new(100);

                tracing::info!("🔄 Starting main polling loop");

                while interval.next().await.is_some() {
                    // 1. Process commands
                    let commands = session_state_clone.borrow_mut().drain_commands();
                    for cmd in commands {
                        if let Err(e) = session_loop.submit_command(cmd) {
                            tracing::error!("❌ Command failed: {:?}", e);
                        }
                    }

                    // 2. Poll
                    session_loop.poll();

                    // 3. Update state (simple - just clone from core)
                    if let Some(l) = session_loop.get_lobby() {
                        lobby_clone.set(Some(l.clone()));
                    }
                    peer_count_clone.set(session_loop.connected_peers().len());
                }

                tracing::warn!("🛑 Polling loop ended");
            });

            move || {
                tracing::info!("🧹 SessionProvider cleanup");
            }
        });
    }

    let context = SessionContext {
        session_id: (*actual_session_id).clone(),
        lobby: (*lobby).clone(),
        peer_count: *peer_count,
        is_host: *is_host,
        send_command,
        local_participant_name: (*local_participant_name).clone(),
    };

    html! {
        <ContextProvider<SessionContext> {context}>
            {props.children.clone()}
        </ContextProvider<SessionContext>>
    }
}
