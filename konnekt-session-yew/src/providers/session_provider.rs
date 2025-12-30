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
    let local_participant_id = use_state(|| None::<uuid::Uuid>);

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
        let is_host = is_host.clone();
        let actual_session_id = actual_session_id.clone();
        let lobby = lobby.clone();
        let peer_count = peer_count.clone();
        let local_participant_id = local_participant_id.clone();
        let session_state = session_state.clone();

        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let ice_servers = IceServer::default_stun_servers();

                let (mut session_loop, sid) = if let Some(sid_str) = session_id_prop {
                    let sid = SessionId::parse(&sid_str).expect("Invalid session ID");
                    tracing::info!("🔗 Joining session: {}", sid);

                    let (mut loop_, lobby_id) = P2PLoopBuilder::new()
                        .build_session_guest(&signalling_server, sid.clone(), ice_servers)
                        .await
                        .expect("Failed to join session");

                    tracing::info!("⏳ Waiting for lobby sync...");

                    // Wait for lobby sync
                    for i in 0..100 {
                        loop_.poll();
                        if loop_.get_lobby().is_some() {
                            tracing::info!("✅ Lobby synced after {} attempts!", i + 1);
                            break;
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

                    is_host.set(false);
                    (loop_, sid)
                } else {
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

                    is_host.set(true);
                    (loop_, sid)
                };

                actual_session_id.set(sid);

                let mut interval = gloo_timers::future::IntervalStream::new(100);
                let mut last_participant_count = 0;

                while interval.next().await.is_some() {
                    // 1. Process commands
                    let commands = session_state.borrow_mut().drain_commands();
                    for cmd in commands {
                        tracing::debug!("📤 Command: {:?}", cmd);
                        if let Err(e) = session_loop.submit_command(cmd) {
                            tracing::error!("❌ Command failed: {:?}", e);
                        }
                    }

                    // 2. Poll
                    session_loop.poll();

                    // 3. Update state
                    if let Some(l) = session_loop.get_lobby() {
                        let current_count = l.participants().len();

                        // Log participant changes
                        if current_count != last_participant_count {
                            tracing::info!(
                                "👥 Participants changed: {} -> {}",
                                last_participant_count,
                                current_count
                            );
                            last_participant_count = current_count;
                        }

                        // ✅ ALWAYS update (Yew handles deduplication)
                        lobby.set(Some(l.clone()));

                        // Find local participant
                        if local_participant_id.is_none() {
                            let current_is_host = *is_host;
                            for p in l.participants().values() {
                                if (current_is_host && p.is_host())
                                    || (!current_is_host && !p.is_host())
                                {
                                    tracing::info!("✅ Local participant: {}", p.name());
                                    local_participant_id.set(Some(p.id()));
                                    break;
                                }
                            }
                        }
                    }

                    peer_count.set(session_loop.connected_peers().len());
                }
            });

            move || {}
        });
    }

    let context = SessionContext {
        session_id: (*actual_session_id).clone(),
        lobby: (*lobby).clone(),
        peer_count: *peer_count,
        is_host: *is_host,
        send_command,
        local_participant_id: *local_participant_id,
    };

    html! {
        <ContextProvider<SessionContext> {context}>
            {props.children.clone()}
        </ContextProvider<SessionContext>>
    }
}
