use clap::{Parser, Subcommand};
use konnekt_session_cli::presentation::tui::{self, App, AppEvent, UserAction};
use konnekt_session_cli::{CliError, Result};
use konnekt_session_core::DomainCommand;
use konnekt_session_p2p::{IceServer, P2PLoopBuilder, SessionId, SessionLoop};
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

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
    let (session_loop, session_id) = P2PLoopBuilder::new()
        .build_session_host(
            server,
            ice_servers,
            "TUI Lobby".to_string(),
            name.to_string(),
        )
        .await?;

    tracing::info!("✅ Session created: {}", session_id);

    run_tui(session_loop, session_id).await
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

    // Wait for lobby to sync from host
    wait_for_lobby_sync(&mut session_loop).await?;

    // Submit join command (business logic in SessionLoop)
    session_loop.submit_command(DomainCommand::JoinLobby {
        lobby_id,
        guest_name: name.to_string(),
    })?;

    tracing::info!("📤 Sent join request");

    run_tui(session_loop, session_id).await
}

async fn run_tui(mut session_loop: SessionLoop, session_id: SessionId) -> Result<()> {
    let mut terminal = tui::setup_terminal()?;
    let mut app = App::new(session_id.to_string());

    let (ui_tx, mut ui_rx) = mpsc::channel(100);
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<UserCommand>(100);

    let lobby_id = session_loop.lobby_id();

    // Spawn SessionLoop background task (ALL BUSINESS LOGIC)
    let session_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // 1. Process user commands from TUI
                    while let Ok(user_cmd) = cmd_rx.try_recv() {
                        if let Err(e) = handle_user_command(&mut session_loop, lobby_id, user_cmd) {
                            tracing::error!("Failed to handle user command: {:?}", e);
                        }
                    }

                    // 2. Poll SessionLoop (business logic)
                    let processed = session_loop.poll();

                    if processed > 0 {
                        tracing::trace!("SessionLoop processed {} events", processed);
                    }

                    // 3. Send UI updates (read-only snapshots)
                    if let Some(lobby) = session_loop.get_lobby() {
                        let _ = ui_tx.send(UiUpdate::Lobby(lobby.clone())).await;
                    }

                    if let Some(peer_id) = session_loop.local_peer_id() {
                        let peer_count = session_loop.connected_peers().len();
                        let is_host = session_loop.is_host();
                        let _ = ui_tx.send(UiUpdate::PeerInfo {
                            peer_id: peer_id.to_string(),
                            peer_count,
                            is_host,
                        }).await;
                    }
                }
            }
        }
    });

    // Run TUI event loop (PRESENTATION ONLY)
    let result = run_app_loop(&mut terminal, &mut app, &mut ui_rx, cmd_tx).await;

    tui::restore_terminal(terminal)?;
    session_handle.abort();

    result
}

/// Commands from TUI to SessionLoop
#[derive(Debug, Clone)]
enum UserCommand {
    ToggleParticipationMode { participant_id: Uuid },
    KickGuest { guest_id: Uuid },
    LeaveSession { participant_id: Uuid },
}

/// Handle user commands (business logic)
fn handle_user_command(
    session_loop: &mut SessionLoop,
    lobby_id: Uuid,
    command: UserCommand,
) -> Result<()> {
    match command {
        UserCommand::ToggleParticipationMode { participant_id } => {
            session_loop.submit_command(DomainCommand::ToggleParticipationMode {
                lobby_id,
                participant_id,
                requester_id: participant_id,
                activity_in_progress: false,
            })?;
            tracing::info!("Submitted toggle participation mode command");
        }
        UserCommand::KickGuest { guest_id } => {
            let host_id = session_loop
                .get_lobby()
                .map(|l| l.host_id())
                .ok_or_else(|| CliError::InvalidConfig("No lobby".to_string()))?;

            session_loop.submit_command(DomainCommand::KickGuest {
                lobby_id,
                host_id,
                guest_id,
            })?;
            tracing::info!("Submitted kick guest command");
        }
        UserCommand::LeaveSession { participant_id } => {
            session_loop.submit_command(DomainCommand::LeaveLobby {
                lobby_id,
                participant_id,
            })?;
            tracing::info!("Submitted leave session command");
        }
    }
    Ok(())
}

/// Updates sent from SessionLoop to TUI
#[derive(Debug, Clone)]
enum UiUpdate {
    Lobby(konnekt_session_core::Lobby),
    PeerInfo {
        peer_id: String,
        peer_count: usize,
        is_host: bool,
    },
}

async fn run_app_loop(
    terminal: &mut tui::TuiTerminal,
    app: &mut App,
    ui_rx: &mut mpsc::Receiver<UiUpdate>,
    cmd_tx: mpsc::Sender<UserCommand>,
) -> Result<()> {
    loop {
        terminal.draw(|f| tui::ui::render(f, app))?;

        tokio::select! {
            Ok(app_event) = tui::event::read_events() => {
                match app_event {
                    AppEvent::Key(key) => {
                        if let Some(action) = app.handle_key(key) {
                            handle_user_action(app, action, &cmd_tx).await?;
                        }
                        if app.should_quit {
                            break;
                        }
                    }
                    AppEvent::Tick => {
                        app.tick();
                    }
                }
            }

            Some(update) = ui_rx.recv() => {
                match update {
                    UiUpdate::Lobby(lobby) => {
                        app.update_lobby(lobby);
                    }
                    UiUpdate::PeerInfo { peer_id, peer_count, is_host } => {
                        app.update_peer_info(peer_id, peer_count, is_host);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Handle user actions (presentation-only side effects + send commands)
async fn handle_user_action(
    app: &mut App,
    action: UserAction,
    cmd_tx: &mpsc::Sender<UserCommand>,
) -> Result<()> {
    match action {
        UserAction::CopySessionId => {
            let _ = app.copy_session_id();
        }
        UserAction::CopyJoinCommand => {
            let _ = app.copy_join_command();
        }
        UserAction::ToggleParticipationMode => {
            if let Some(participant_id) = app.get_local_participant_id() {
                cmd_tx
                    .send(UserCommand::ToggleParticipationMode { participant_id })
                    .await
                    .map_err(|e| {
                        CliError::InvalidConfig(format!("Failed to send command: {}", e))
                    })?;
            } else {
                tracing::warn!("Cannot toggle mode: participant ID not known yet");
            }
        }
        UserAction::KickParticipant(guest_id) => {
            cmd_tx
                .send(UserCommand::KickGuest { guest_id })
                .await
                .map_err(|e| CliError::InvalidConfig(format!("Failed to send command: {}", e)))?;
        }
        UserAction::Quit => {
            // Send leave command if we're a guest
            if !app.is_host {
                if let Some(participant_id) = app.get_local_participant_id() {
                    let _ = cmd_tx
                        .send(UserCommand::LeaveSession { participant_id })
                        .await;
                }
            }
        }
    }
    Ok(())
}

async fn wait_for_lobby_sync(session_loop: &mut SessionLoop) -> Result<()> {
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    tracing::info!("⏳ Waiting for lobby sync from host...");

    while start.elapsed() < timeout {
        session_loop.poll();

        if session_loop.get_lobby().is_some() {
            tracing::info!("✅ Lobby synced!");
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err(CliError::P2PConnection(
        "Timeout waiting for lobby sync".to_string(),
    ))
}
