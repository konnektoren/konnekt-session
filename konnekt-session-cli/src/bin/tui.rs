use clap::{Parser, Subcommand};
use konnekt_session_cli::presentation::tui::{self, App, AppEvent};
use konnekt_session_cli::{CliError, Result, domain::SessionState}; // 🆕 FIXED: Import from root
use konnekt_session_core::{DomainCommand, Participant};
use konnekt_session_p2p::{IceServer, P2PLoopBuilder, SessionId, SessionLoop};
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
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

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
    tracing::info!("🎯 Creating host session...");

    // Build SessionLoop as host
    let (mut session_loop, session_id) = P2PLoopBuilder::new()
        .build_session_host(
            server,
            ice_servers,
            "TUI Lobby".to_string(),
            name.to_string(),
        )
        .await?;

    let lobby_id = session_loop.lobby_id();

    tracing::info!("✅ Session created: {}", session_id);
    tracing::info!("📋 Lobby ID: {}", lobby_id);

    // Wait for peer ID
    wait_for_peer_id(&mut session_loop).await?;

    // Get the created lobby
    let lobby = session_loop
        .get_lobby()
        .ok_or_else(|| CliError::P2PConnection("Lobby not created".to_string()))?
        .clone();

    tracing::info!("✅ Lobby ready: {}", lobby.name());

    let host = Participant::new_host(name.to_string())?; // Auto-converts via From<ParticipantError>

    let mut state = SessionState::new(host);
    state.set_lobby(lobby);

    run_tui(session_loop, state, session_id, true).await
}

async fn join_session(
    server: &str,
    session_id_str: &str,
    name: &str,
    ice_servers: Vec<IceServer>,
) -> Result<()> {
    tracing::info!("🎯 Joining session...");

    let session_id = SessionId::parse(session_id_str)?;

    let (mut session_loop, lobby_id) = P2PLoopBuilder::new()
        .build_session_guest(server, session_id.clone(), ice_servers)
        .await?;

    tracing::info!("✅ Joined session: {}", session_id);

    // Wait for peer ID
    wait_for_peer_id(&mut session_loop).await?;

    // Wait for peer connection
    wait_for_peer_connection(&mut session_loop).await?;

    // Wait for lobby sync (host will send automatically)
    tracing::info!("⏳ Waiting for lobby sync from host...");
    wait_for_lobby_sync(&mut session_loop).await?;

    // Submit join command
    session_loop.submit_command(DomainCommand::JoinLobby {
        lobby_id,
        guest_name: name.to_string(),
    })?;

    tracing::info!("📤 Sent join request");

    // Give it a moment to process
    tokio::time::sleep(Duration::from_millis(200)).await;
    session_loop.poll();

    let guest = Participant::new_guest(name.to_string())?;

    let state = SessionState::new(guest);

    run_tui(session_loop, state, session_id, false).await
}

async fn run_tui(
    mut session_loop: SessionLoop,
    state: SessionState,
    session_id: SessionId,
    is_host: bool,
) -> Result<()> {
    let mut terminal = tui::setup_terminal()?;
    let session_id_str = session_id.to_string();
    let mut app = App::new(state, session_id_str);

    let (state_tx, mut state_rx) = mpsc::channel(100);

    // Spawn background session loop task
    let session_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Poll SessionLoop (auto-syncs P2P ↔ Core)
                    let processed = session_loop.poll();

                    if processed > 0 {
                        tracing::trace!("SessionLoop processed {} events", processed);
                    }

                    // Send lobby state to UI
                    if let Some(lobby) = session_loop.get_lobby() {
                        let _ = state_tx.send(AppUpdate::LobbyState(lobby.clone())).await;
                    }

                    // Send peer count
                    let peer_count = session_loop.connected_peers().len();
                    let _ = state_tx.send(AppUpdate::PeerCount(peer_count)).await;
                }
            }
        }
    });

    // Run TUI event loop
    let result = run_app_loop(&mut terminal, &mut app, &mut state_rx).await;

    tui::restore_terminal(terminal)?;

    // Cleanup
    session_handle.abort();

    result
}

/// Updates sent from SessionLoop to TUI
enum AppUpdate {
    LobbyState(konnekt_session_core::Lobby),
    PeerCount(usize),
}

async fn run_app_loop(
    terminal: &mut tui::TuiTerminal,
    app: &mut App,
    state_rx: &mut mpsc::Receiver<AppUpdate>,
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

            Some(update) = state_rx.recv() => {
                match update {
                    AppUpdate::LobbyState(lobby) => {
                        app.update_lobby(lobby);
                    }
                    AppUpdate::PeerCount(count) => {
                        app.update_peer_count(count);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn wait_for_peer_id(session_loop: &mut SessionLoop) -> Result<()> {
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    tracing::info!("⏳ Waiting for peer ID...");

    while start.elapsed() < timeout {
        session_loop.poll();

        if session_loop.local_peer_id().is_some() {
            tracing::info!("✅ Peer ID assigned");
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    Err(CliError::P2PConnection(
        "Timeout waiting for peer ID".to_string(),
    ))
}

/// Wait for at least one peer to connect
async fn wait_for_peer_connection(session_loop: &mut SessionLoop) -> Result<()> {
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    tracing::info!("⏳ Waiting for peer connection...");

    while start.elapsed() < timeout {
        session_loop.poll();

        if !session_loop.connected_peers().is_empty() {
            tracing::info!(
                "✅ Connected to {} peer(s)",
                session_loop.connected_peers().len()
            );

            // Give the connection a moment to fully establish
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Poll again to let host detect the connection
            session_loop.poll();

            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err(CliError::P2PConnection(
        "Timeout waiting for peer connection".to_string(),
    ))
}

async fn wait_for_lobby_sync(session_loop: &mut SessionLoop) -> Result<()> {
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        // Poll to process incoming messages
        let processed = session_loop.poll();

        if processed > 0 {
            tracing::debug!("Processed {} events during sync wait", processed);
        }

        // Check if we received lobby via P2P sync
        if let Some(lobby) = session_loop.get_lobby() {
            tracing::info!("✅ Lobby '{}' synced!", lobby.name());
            tracing::info!("   Host: {:?}", lobby.host_id());
            tracing::info!("   Participants: {}", lobby.participants().len());
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    tracing::error!("❌ Timeout waiting for lobby sync");
    tracing::error!("   Lobby ID: {}", session_loop.lobby_id());
    tracing::error!(
        "   Connected peers: {}",
        session_loop.connected_peers().len()
    );

    Err(CliError::P2PConnection(format!(
        "Timeout waiting for lobby {} to sync",
        session_loop.lobby_id()
    )))
}
