use crate::hooks::{ActiveRunSnapshot, SessionContext};
use bevy_ecs::prelude::{Resource, World};
use bevy_ecs::schedule::Schedule;
use bevy_ecs::system::ResMut;
use futures::StreamExt;
use konnekt_session_core::{DomainCommand, Lobby};
use konnekt_session_p2p::{IceServer, P2PLoopBuilder, SessionId, SessionLoop};
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SessionProviderProps {
    pub signalling_server: AttrValue,
    #[prop_or_default]
    pub lobby_name: Option<AttrValue>,
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

#[derive(Resource)]
struct RuntimeState {
    session_loop: SessionLoop,
    is_host: bool,
    lobby_id: Uuid,
    local_name: String,
    sync_retry_ticks: u16,
    join_retry_ticks: u16,
    /// True while we are waiting for the host to acknowledge our JoinLobby
    /// (i.e. the GuestJoined broadcast to arrive back). Prevents sending
    /// duplicate JoinLobby commands while the round-trip is in flight.
    join_in_flight: bool,
}

#[derive(Resource, Default)]
struct PendingCommands(Vec<DomainCommand>);

#[derive(Resource, Clone, Default)]
struct RuntimeSnapshot {
    lobby: Option<Lobby>,
    active_run: Option<ActiveRunSnapshot>,
    peer_count: usize,
    local_participant_id: Option<Uuid>,
    local_peer_id: Option<String>,
}

fn drive_session_runtime(
    mut state: ResMut<RuntimeState>,
    mut pending_commands: ResMut<PendingCommands>,
    mut snapshot: ResMut<RuntimeSnapshot>,
) {
    for cmd in pending_commands.0.drain(..) {
        if let Err(e) = state.session_loop.submit_command(cmd) {
            tracing::error!("❌ Command failed: {:?}", e);
        }
    }

    let processed = state.session_loop.poll();
    if processed > 0 {
        tracing::debug!("SessionRuntime processed {} events", processed);
    }

    // Guest resiliency:
    // 1) periodically request full sync until lobby arrives
    // 2) periodically retry JoinLobby once at least one peer is connected
    //    (don't wait for lobby snapshot, to avoid sync deadlocks)
    if !state.is_host {
        let has_connected_peers = !state.session_loop.connected_peers().is_empty();
        let has_lobby = state.session_loop.get_lobby().is_some();

        // ── 1. Sync retry: keep requesting full state until lobby arrives ────
        if has_connected_peers && !has_lobby {
            state.sync_retry_ticks = state.sync_retry_ticks.saturating_add(1);
            if state.sync_retry_ticks >= 20 {
                state.sync_retry_ticks = 0;
                if let Err(e) = state.session_loop.p2p_mut().request_full_sync() {
                    tracing::warn!("⚠️ Retry full sync request failed: {:?}", e);
                } else {
                    tracing::info!("🔁 Retried full sync request");
                }
            }
        } else {
            state.sync_retry_ticks = 0;
        }

        // ── 2. Determine whether the guest is present in the lobby ──────────
        // Check by participant_id first (most accurate), fall back to name-match
        // for the brief window before local_participant_id is resolved.
        let joined = has_lobby && {
            let by_id = snapshot.local_participant_id.and_then(|id| {
                state
                    .session_loop
                    .get_lobby()
                    .and_then(|l| l.participants().get(&id).map(|_| true))
            });
            by_id.unwrap_or_else(|| {
                state
                    .session_loop
                    .get_lobby()
                    .map(|l| {
                        l.participants()
                            .values()
                            .any(|p| p.name() == state.local_name && !p.is_host())
                    })
                    .unwrap_or(false)
            })
        };

        // Once confirmed joined, clear the in-flight flag so we don't re-send.
        if joined {
            state.join_in_flight = false;
        }

        // ── 3. JoinLobby retry ───────────────────────────────────────────────
        // Send JoinLobby whenever we have peers but are not yet in the lobby.
        // This covers two cases:
        //   a) No lobby yet  — keep retrying every ~1s until snapshot arrives.
        //   b) Lobby arrived without the guest (host snapshotted before processing
        //      the initial join) — retry until the GuestJoined ack comes back.
        // The `join_in_flight` flag suppresses duplicate sends while the
        // round-trip (guest → host → GuestJoined broadcast → guest) is in flight.
        if has_connected_peers && !joined && !state.join_in_flight {
            state.join_retry_ticks = state.join_retry_ticks.saturating_add(1);
            if state.join_retry_ticks >= 10 {
                state.join_retry_ticks = 0;
                let lobby_id = state.lobby_id;
                let guest_name = state.local_name.clone();
                if let Err(e) = state.session_loop.submit_command(DomainCommand::JoinLobby {
                    lobby_id,
                    guest_name: guest_name.clone(),
                }) {
                    tracing::warn!("⚠️ JoinLobby failed: {:?}", e);
                } else {
                    tracing::info!("🔁 Sent JoinLobby for '{}' (in-flight)", guest_name);
                    state.join_in_flight = true;
                }
            }
        } else if !has_connected_peers {
            // Reset tick counter while disconnected so we retry quickly on reconnect.
            state.join_retry_ticks = 0;
        }
    }

    *snapshot = RuntimeSnapshot {
        lobby: state.session_loop.get_lobby().cloned(),
        active_run: state
            .session_loop
            .get_lobby()
            .and_then(|l| l.active_run_id())
            .and_then(|run_id| state.session_loop.domain().event_loop().get_run(&run_id))
            .map(|run| ActiveRunSnapshot {
                run_id: run.id(),
                status: run.status(),
                name: run.config().name.clone(),
                config: run.config().config.clone(),
                required_submitters: run.required_submitters().iter().copied().collect(),
                results: run.results().values().cloned().collect(),
            }),
        peer_count: state.session_loop.connected_peers().len(),
        local_participant_id: state
            .session_loop
            .local_peer_id()
            .and_then(|peer_id| state.session_loop.p2p().peer_registry().get_peer(&peer_id))
            .and_then(|peer_state| peer_state.participant_id),
        local_peer_id: state.session_loop.local_peer_id().map(|p| p.to_string()),
    };
}

fn parse_session_reference(raw: &str) -> Option<SessionId> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(id) = SessionId::parse(trimmed) {
        return Some(id);
    }

    let without_query = trimmed.split('?').next().unwrap_or(trimmed);
    let candidate = without_query.trim_end_matches('/');
    let tail = candidate.rsplit('/').next().unwrap_or(candidate);
    SessionId::parse(tail).ok()
}

#[function_component(SessionProvider)]
pub fn session_provider(props: &SessionProviderProps) -> Html {
    let starts_as_host = props.session_id.is_none();
    let lobby = use_state(|| None::<Lobby>);
    let active_run = use_state(|| None::<ActiveRunSnapshot>);
    let peer_count = use_state(|| 0usize);
    let local_participant_id = use_state(|| None::<Uuid>);
    let local_peer_id = use_state(|| None::<String>);
    let is_host = use_state(move || starts_as_host);
    let actual_session_id = use_state(|| SessionId::new());
    let local_participant_name = use_state(|| None::<String>);
    let runtime_error = use_state(|| None::<String>);

    let session_state = use_mut_ref(SessionState::new);

    let send_command = {
        let session_state = session_state.clone();
        Rc::new(move |cmd: DomainCommand| {
            session_state.borrow_mut().enqueue_command(cmd);
        }) as Rc<dyn Fn(DomainCommand)>
    };

    {
        let signalling_server = props.signalling_server.to_string();
        let lobby_name = props
            .lobby_name
            .clone()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "Yew Lobby".to_string());
        let session_id_prop = props.session_id.clone();
        let name = props.name.clone().unwrap_or_else(|| "Guest".into());
        let is_host_clone = is_host.clone();
        let actual_session_id_clone = actual_session_id.clone();
        let lobby_clone = lobby.clone();
        let active_run_clone = active_run.clone();
        let peer_count_clone = peer_count.clone();
        let local_participant_id_clone = local_participant_id.clone();
        let local_peer_id_clone = local_peer_id.clone();
        let local_participant_name_clone = local_participant_name.clone();
        let runtime_error_clone = runtime_error.clone();
        let session_state_clone = session_state.clone();

        use_effect_with((), move |_| {
            tracing::info!("🚀 SessionProvider starting");

            wasm_bindgen_futures::spawn_local(async move {
                let ice_servers = IceServer::default_stun_servers();
                let local_name = name.to_string();

                let (session_loop, sid) = if let Some(sid_str) = session_id_prop {
                    let sid = match parse_session_reference(&sid_str) {
                        Some(parsed) => parsed,
                        None => {
                            let msg = format!(
                                "Invalid session reference '{}'. Expected UUID or room URL ending with UUID.",
                                sid_str
                            );
                            tracing::error!("❌ {}", msg);
                            runtime_error_clone.set(Some(msg));
                            return;
                        }
                    };
                    tracing::info!("🔗 Joining session: {}", sid);

                    let (loop_, _lobby_id) = match P2PLoopBuilder::new()
                        .build_session_guest(&signalling_server, sid.clone(), ice_servers)
                        .await
                    {
                        Ok(v) => v,
                        Err(e) => {
                            let msg = format!("Failed to join session {}: {:?}", sid, e);
                            tracing::error!("❌ {}", msg);
                            runtime_error_clone.set(Some(msg));
                            return;
                        }
                    };

                    // Store our name
                    local_participant_name_clone.set(Some(local_name.clone()));
                    is_host_clone.set(false);

                    (loop_, sid)
                } else {
                    // Host creation
                    tracing::info!("👑 Creating host session as '{}'", name);

                    let (loop_, sid) = match P2PLoopBuilder::new()
                        .build_session_host(
                            &signalling_server,
                            ice_servers,
                            lobby_name.clone(),
                            local_name.clone(),
                        )
                        .await
                    {
                        Ok(v) => v,
                        Err(e) => {
                            let msg = format!("Failed to create host session: {:?}", e);
                            tracing::error!("❌ {}", msg);
                            runtime_error_clone.set(Some(msg));
                            return;
                        }
                    };

                    // Store our name
                    local_participant_name_clone.set(Some(local_name.clone()));
                    is_host_clone.set(true);

                    (loop_, sid)
                };

                actual_session_id_clone.set(sid);
                runtime_error_clone.set(None);

                // Run the session through a Bevy ECS application tick.
                let runtime_is_host = session_loop.is_host();
                let runtime_lobby_id = session_loop.lobby_id();
                let mut world = World::new();
                world.insert_resource(RuntimeState {
                    session_loop,
                    is_host: runtime_is_host,
                    lobby_id: runtime_lobby_id,
                    local_name,
                    // Make first retry happen quickly on startup.
                    sync_retry_ticks: 19,
                    join_retry_ticks: 9,
                    join_in_flight: false,
                });
                world.insert_resource(PendingCommands::default());
                world.insert_resource(RuntimeSnapshot::default());

                let mut schedule = Schedule::default();
                schedule.add_systems(drive_session_runtime);

                let mut interval = gloo_timers::future::IntervalStream::new(100);

                tracing::info!("🔄 Starting main polling loop");

                while interval.next().await.is_some() {
                    // 1. Drain Yew command queue into Bevy resources
                    let commands = session_state_clone.borrow_mut().drain_commands();
                    world.resource_mut::<PendingCommands>().0.extend(commands);

                    // 2. Run one Bevy ECS tick (synchronous — blocks JS event loop)
                    schedule.run(&mut world);

                    // Yield for a few ms so the WebRTC loop_fut gets multiple event-loop
                    // turns to process ICE candidates, DTLS handshakes, and signaling
                    // messages. A single 0ms yield is not enough for ICE negotiation.
                    gloo_timers::future::TimeoutFuture::new(5).await;

                    // 3. Publish snapshot to Yew state — only set when changed to avoid render spam
                    let snapshot = world.resource::<RuntimeSnapshot>().clone();
                    if *lobby_clone != snapshot.lobby {
                        lobby_clone.set(snapshot.lobby);
                    }
                    if *active_run_clone != snapshot.active_run {
                        active_run_clone.set(snapshot.active_run);
                    }
                    if *peer_count_clone != snapshot.peer_count {
                        peer_count_clone.set(snapshot.peer_count);
                    }
                    if *local_participant_id_clone != snapshot.local_participant_id {
                        local_participant_id_clone.set(snapshot.local_participant_id);
                    }
                    if *local_peer_id_clone != snapshot.local_peer_id {
                        local_peer_id_clone.set(snapshot.local_peer_id);
                    }
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
        active_run: (*active_run).clone(),
        local_participant_id: *local_participant_id,
        local_peer_id: (*local_peer_id).clone(),
        send_command,
        local_participant_name: (*local_participant_name).clone(),
        runtime_error: (*runtime_error).clone(),
    };

    html! {
        <ContextProvider<SessionContext> {context}>
            {props.children.clone()}
        </ContextProvider<SessionContext>>
    }
}
