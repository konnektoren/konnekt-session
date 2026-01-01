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
        let is_host_clone = is_host.clone();
        let actual_session_id_clone = actual_session_id.clone();
        let lobby_clone = lobby.clone();
        let peer_count_clone = peer_count.clone();
        let local_participant_id_clone = local_participant_id.clone();
        let session_state_clone = session_state.clone();

        // ✅ FIX: Only run ONCE on mount
        use_effect_with((), move |_| {
            tracing::info!("🚀 SessionProvider effect starting (should only run ONCE)");

            wasm_bindgen_futures::spawn_local(async move {
                let ice_servers = IceServer::default_stun_servers();

                let (mut session_loop, sid) = if let Some(sid_str) = session_id_prop {
                    let sid = SessionId::parse(&sid_str).expect("Invalid session ID");
                    tracing::info!("🔗 Joining session: {}", sid);

                    let (mut loop_, lobby_id) = P2PLoopBuilder::new()
                        .build_session_guest(&signalling_server, sid.clone(), ice_servers)
                        .await
                        .expect("Failed to join session");

                    // 🔥 NEW: Wait for peer connection BEFORE doing anything else
                    tracing::info!("⏳ Waiting for peer connection...");
                    for i in 0..100 {
                        loop_.poll();
                        if !loop_.connected_peers().is_empty() {
                            tracing::info!(
                                "✅ Connected to {} peer(s) after {} attempts",
                                loop_.connected_peers().len(),
                                i + 1
                            );
                            break;
                        }

                        if i == 99 {
                            tracing::error!(
                                "❌ Timeout: No peer connection established after 10 seconds"
                            );
                            tracing::error!("   Make sure the host is running and reachable");
                        }

                        gloo_timers::future::TimeoutFuture::new(100).await;
                    }

                    // 🔥 NEW: Additional wait for lobby sync from host
                    tracing::info!("⏳ Waiting for lobby sync from host...");
                    for i in 0..100 {
                        loop_.poll();
                        if loop_.get_lobby().is_some() {
                            tracing::info!("✅ Lobby synced after {} attempts!", i + 1);
                            break;
                        }

                        if i == 99 {
                            tracing::error!("❌ Timeout: Lobby never synced from host");
                            tracing::error!("   Host should auto-send snapshot on connection");
                        }

                        gloo_timers::future::TimeoutFuture::new(100).await;
                    }

                    // NOW we can send commands - we have a peer connection AND lobby state
                    tracing::info!("📤 Submitting JoinLobby as '{}'", name);
                    if let Err(e) = loop_.submit_command(DomainCommand::JoinLobby {
                        lobby_id,
                        guest_name: name.to_string(),
                    }) {
                        tracing::error!("❌ Failed to join: {:?}", e);
                    }

                    // Poll until we see ourselves in the lobby
                    let name_str = name.to_string();
                    for i in 0..50 {
                        loop_.poll();

                        if let Some(lobby) = loop_.get_lobby() {
                            if let Some(our_participant) = lobby
                                .participants()
                                .values()
                                .find(|p| p.name() == name_str.as_str() && !p.is_host())
                            {
                                tracing::info!(
                                    "✅ Found ourselves in lobby: {} (id: {})",
                                    our_participant.name(),
                                    our_participant.id()
                                );
                                local_participant_id_clone.set(Some(our_participant.id()));
                                break;
                            }
                        }

                        if i == 49 {
                            tracing::error!(
                                "❌ Timeout: Didn't find ourselves in lobby after 5 seconds"
                            );
                            if let Some(lobby) = loop_.get_lobby() {
                                tracing::error!("   Current participants:");
                                for p in lobby.participants().values() {
                                    tracing::error!(
                                        "     - {} (id: {}, host: {})",
                                        p.name(),
                                        p.id(),
                                        p.is_host()
                                    );
                                }
                            }
                        }

                        gloo_timers::future::TimeoutFuture::new(100).await;
                    }

                    is_host_clone.set(false);
                    (loop_, sid)
                } else {
                    tracing::info!("👑 Creating host session as '{}'", name);

                    let (mut loop_, sid) = P2PLoopBuilder::new()
                        .build_session_host(
                            &signalling_server,
                            ice_servers,
                            "Yew Lobby".to_string(),
                            name.to_string(),
                        )
                        .await
                        .expect("Failed to create session");

                    if let Some(lobby) = loop_.get_lobby() {
                        if let Some(host_participant) =
                            lobby.participants().values().find(|p| p.is_host())
                        {
                            tracing::info!(
                                "✅ Host participant: {} (id: {})",
                                host_participant.name(),
                                host_participant.id()
                            );
                            local_participant_id_clone.set(Some(host_participant.id()));
                        }
                    }

                    is_host_clone.set(true);
                    (loop_, sid)
                };

                actual_session_id_clone.set(sid);

                let mut interval = gloo_timers::future::IntervalStream::new(100);
                let name_str = name.to_string();
                let current_is_host = *is_host_clone;

                tracing::info!("🔄 Starting main polling loop");

                while interval.next().await.is_some() {
                    // 1. Process commands
                    let commands = session_state_clone.borrow_mut().drain_commands();
                    if !commands.is_empty() {
                        tracing::debug!("📤 Processing {} commands", commands.len());
                    }
                    for cmd in commands {
                        if let Err(e) = session_loop.submit_command(cmd) {
                            tracing::error!("❌ Command failed: {:?}", e);
                        }
                    }

                    // 2. Poll
                    session_loop.poll();

                    // 3. Update state (BATCHED to reduce re-renders)
                    if let Some(l) = session_loop.get_lobby() {
                        let current_count = l.participants().len();
                        let old_participant_id = *local_participant_id_clone;

                        // 🔥 ONLY update participant ID if it's actually different
                        let mut new_participant_id = old_participant_id;

                        for p in l.participants().values() {
                            if p.name() == name_str.as_str() && p.is_host() == current_is_host {
                                new_participant_id = Some(p.id());
                                break;
                            }
                        }

                        // ✅ Only set if changed (prevents infinite loops)
                        if old_participant_id != new_participant_id {
                            tracing::info!(
                                "🔄 Participant ID changed: {:?} → {:?}",
                                old_participant_id,
                                new_participant_id
                            );
                            local_participant_id_clone.set(new_participant_id);
                        }

                        // ✅ Clone lobby ONCE per poll (not per state update)
                        let lobby_clone_data = l.clone();
                        let new_peer_count = session_loop.connected_peers().len();

                        // ✅ Batch all state updates together
                        lobby_clone.set(Some(lobby_clone_data));
                        peer_count_clone.set(new_peer_count);
                    }
                }

                tracing::warn!("🛑 Polling loop ended (should never happen)");
            });

            // ✅ Cleanup function (runs when component unmounts)
            move || {
                tracing::info!("🧹 SessionProvider cleanup (component unmounting)");
            }
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
