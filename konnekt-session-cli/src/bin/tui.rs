use clap::{Parser, Subcommand};
use konnekt_session_cli::{
    domain::SessionState,
    infrastructure::{CliError, Result},
    presentation::tui::{self, App, AppEvent},
};
use konnekt_session_core::{DomainCommand, Participant};
use konnekt_session_p2p::{ConnectionEvent, IceServer, P2PLoopBuilder, SessionId};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(name = "konnekt-tui")]
#[command(
    version,
    about = "Konnekt Session TUI - Interactive terminal interface"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    CreateHost {
        #[arg(short = 's', long, default_value = "wss://match.konnektoren.help")]
        server: String,
        #[arg(short = 'n', long, default_value = "Host")]
        name: String,
        #[arg(long)]
        turn_server: Option<String>,
        #[arg(long)]
        turn_username: Option<String>,
        #[arg(long)]
        turn_credential: Option<String>,
    },
    Join {
        #[arg(short = 's', long, default_value = "wss://match.konnektoren.help")]
        server: String,
        #[arg(short = 'i', long)]
        session_id: String,
        #[arg(short = 'n', long, default_value = "Guest")]
        name: String,
        #[arg(long)]
        turn_server: Option<String>,
        #[arg(long)]
        turn_username: Option<String>,
        #[arg(long)]
        turn_credential: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::CreateHost {
            server,
            name,
            turn_server,
            turn_username,
            turn_credential,
        } => {
            let ice_servers = build_ice_servers(turn_server, turn_username, turn_credential)?;
            create_host(&server, &name, ice_servers).await?;
        }
        Commands::Join {
            server,
            session_id,
            name,
            turn_server,
            turn_username,
            turn_credential,
        } => {
            let ice_servers = build_ice_servers(turn_server, turn_username, turn_credential)?;
            join_session(&server, &session_id, &name, ice_servers).await?;
        }
    }

    Ok(())
}

fn build_ice_servers(
    turn_server: Option<String>,
    turn_username: Option<String>,
    turn_credential: Option<String>,
) -> Result<Vec<IceServer>> {
    let mut ice_servers = IceServer::default_stun_servers();

    if let Some(turn_url) = turn_server {
        match (turn_username, turn_credential) {
            (Some(username), Some(credential)) => {
                ice_servers.push(IceServer::turn(turn_url, username, credential));
            }
            _ => {
                return Err(CliError::InvalidConfig(
                    "TURN server requires both username and credential".to_string(),
                ));
            }
        }
    }

    Ok(ice_servers)
}

async fn create_host(server: &str, name: &str, ice_servers: Vec<IceServer>) -> Result<()> {
    let (mut runtime, session_id, lobby_id) = konnekt_session_cli::RuntimeBuilder::new()
        .build_host(server, ice_servers)
        .await?;

    tracing::info!("✅ Session created: {}", session_id);
    tracing::info!("📋 Lobby ID: {}", lobby_id);

    wait_for_peer_id(&mut runtime).await?;

    // Create lobby in domain
    let lobby = {
        let cmd = DomainCommand::CreateLobby {
            lobby_id: Some(lobby_id),
            lobby_name: "TUI Lobby".to_string(),
            host_name: name.to_string(),
        };

        runtime.domain_loop_mut().submit(cmd)?; // ✅ Now works with From<QueueError>
        runtime.domain_loop_mut().poll();

        match runtime.domain_loop_mut().drain_events().into_iter().next() {
            Some(konnekt_session_core::DomainEvent::LobbyCreated { lobby }) => {
                tracing::info!("✅ Lobby created: {}", lobby.name());
                lobby
            }
            _ => {
                return Err(CliError::InvalidConfig(
                    "Failed to create lobby".to_string(),
                ));
            }
        }
    };

    runtime.set_lobby(lobby_id, true);

    // Initial broadcast (for any peers already connected)
    runtime
        .p2p_loop_mut()
        .broadcast_event(konnekt_session_p2p::DomainEvent::LobbyCreated {
            lobby_id,
            host_id: lobby.host_id(),
            name: lobby.name().to_string(),
        })?;

    tracing::info!("📤 Initial LobbyCreated broadcast");

    let host = Participant::new_host(name.to_string())?; // ✅ Now works with From<ParticipantError>
    let mut state = SessionState::new(host);
    state.set_lobby(lobby.clone());

    // Create snapshot for peer sync
    let lobby_snapshot = konnekt_session_p2p::LobbySnapshot {
        lobby_id,
        name: lobby.name().to_string(),
        host_id: lobby.host_id(),
        participants: lobby.participants().values().cloned().collect(),
        as_of_sequence: runtime.p2p_loop().current_sequence(),
    };

    run_tui(runtime, state, session_id, Some(lobby_snapshot)).await
}

async fn join_session(
    server: &str,
    session_id_str: &str,
    name: &str,
    ice_servers: Vec<IceServer>,
) -> Result<()> {
    let session_id = SessionId::parse(session_id_str)?;

    let (mut runtime, session_id, lobby_id) = konnekt_session_cli::RuntimeBuilder::new()
        .build_guest(server, session_id, ice_servers)
        .await?;

    tracing::info!("✅ Joined session: {}", session_id);

    wait_for_peer_id(&mut runtime).await?;

    // Wait for lobby to sync
    tracing::info!("⏳ Waiting for lobby sync...");
    wait_for_lobby_sync(&mut runtime, lobby_id).await?;

    let guest = Participant::new_guest(name.to_string())?;
    let state = SessionState::new(guest);

    run_tui(runtime, state, session_id, None).await
}

async fn run_tui(
    mut runtime: konnekt_session_cli::DualLoopRuntime,
    state: SessionState,
    session_id: SessionId,
    lobby_snapshot: Option<konnekt_session_p2p::LobbySnapshot>,
) -> Result<()> {
    let mut terminal = tui::setup_terminal()?;
    let session_id_str = session_id.to_string();
    let mut app = App::new(state, session_id_str);

    let (runtime_tx, mut runtime_rx) = mpsc::channel(100);

    // Spawn background runtime task
    let runtime_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(10));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    runtime.tick();

                    // Handle peer connected - send full sync if host
                    if let Some(ref snapshot) = lobby_snapshot {
                        for event in runtime.drain_connection_events() {
                            if let ConnectionEvent::PeerConnected(peer_id) = &event {
                                tracing::info!("📤 Sending full sync to new peer {}", peer_id);

                                if let Err(e) = runtime.p2p_loop_mut().send_full_sync_to_peer(*peer_id, snapshot.clone()) {
                                    tracing::error!("❌ Failed to send full sync: {:?}", e);
                                }
                            }

                            let _ = runtime_tx.send(event).await;
                        }
                    } else {
                        // Guest - just forward events
                        for event in runtime.drain_connection_events() {
                            let _ = runtime_tx.send(event).await;
                        }
                    }
                }
            }
        }
    });

    // Run TUI event loop
    let result = run_app_loop(&mut terminal, &mut app, &mut runtime_rx).await;

    tui::restore_terminal(terminal)?;

    // Cleanup
    runtime_handle.abort();

    result
}

async fn run_app_loop(
    terminal: &mut tui::TuiTerminal,
    app: &mut App,
    runtime_rx: &mut mpsc::Receiver<ConnectionEvent>,
) -> Result<()> {
    loop {
        terminal.draw(|f| tui::ui::render(f, app))?;

        tokio::select! {
            Ok(app_event) = tui::event::read_events() => {
                match app_event {
                    AppEvent::Key(key) => {
                        app.handle_key(key);
                        if app.should_quit {
                            break;
                        }
                    }
                    AppEvent::Tick => {
                        app.tick();
                    }
                }
            }

            Some(event) = runtime_rx.recv() => {
                app.handle_connection_event(&event);
            }
        }
    }

    Ok(())
}

async fn wait_for_peer_id(runtime: &mut konnekt_session_cli::DualLoopRuntime) -> Result<()> {
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        runtime.tick();

        if runtime.p2p_loop().local_peer_id().is_some() {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    Err(CliError::P2PConnection(
        "Timeout waiting for peer ID".to_string(),
    ))
}

async fn wait_for_lobby_sync(
    runtime: &mut konnekt_session_cli::DualLoopRuntime,
    lobby_id: uuid::Uuid,
) -> Result<()> {
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        runtime.tick();

        // Check if we received lobby via P2P sync
        if let Some(lobby) = runtime.domain_loop().event_loop().get_lobby(&lobby_id) {
            tracing::info!("✅ Lobby '{}' synced!", lobby.name());
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err(CliError::P2PConnection(format!(
        "Timeout waiting for lobby {} to sync",
        lobby_id
    )))
}
