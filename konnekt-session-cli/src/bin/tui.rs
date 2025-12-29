use clap::{Parser, Subcommand};
use konnekt_session_cli::{
    application::use_cases::{
        handle_message_received, handle_peer_connected, handle_peer_disconnected,
        handle_peer_timed_out,
    },
    domain::SessionState,
    infrastructure::{CliError, Result},
    presentation::tui::{self, App, AppEvent},
};
use konnekt_session_core::{Lobby, Participant};
use konnekt_session_p2p::{ConnectionEvent, P2PSession, SessionConfig, SessionId};
use std::time::Duration;

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
    /// Create a new session as host
    CreateHost {
        /// Matchbox signalling server URL
        #[arg(short = 's', long, default_value = "wss://match.konnektoren.help")]
        server: String,

        /// Host display name
        #[arg(short = 'n', long, default_value = "Host")]
        name: String,

        /// TURN server URL
        #[arg(long)]
        turn_server: Option<String>,

        /// TURN username
        #[arg(long)]
        turn_username: Option<String>,

        /// TURN credential
        #[arg(long)]
        turn_credential: Option<String>,
    },

    /// Join an existing session as guest
    Join {
        /// Matchbox signalling server URL
        #[arg(short = 's', long, default_value = "wss://match.konnektoren.help")]
        server: String,

        /// Session ID to join
        #[arg(short = 'i', long)]
        session_id: String,

        /// Guest display name
        #[arg(short = 'n', long, default_value = "Guest")]
        name: String,

        /// TURN server URL
        #[arg(long)]
        turn_server: Option<String>,

        /// TURN username
        #[arg(long)]
        turn_username: Option<String>,

        /// TURN credential
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
            let config = build_config(&server, turn_server, turn_username, turn_credential)?;
            create_host(config, &name).await?;
        }
        Commands::Join {
            server,
            session_id,
            name,
            turn_server,
            turn_username,
            turn_credential,
        } => {
            let config = build_config(&server, turn_server, turn_username, turn_credential)?;
            join_session(config, &session_id, &name).await?;
        }
    }

    Ok(())
}

fn build_config(
    server: &str,
    turn_server: Option<String>,
    turn_username: Option<String>,
    turn_credential: Option<String>,
) -> Result<SessionConfig> {
    let mut config = SessionConfig::new(server.to_string());

    if let Some(turn_url) = turn_server {
        match (turn_username, turn_credential) {
            (Some(username), Some(credential)) => {
                config = config.with_turn_server(turn_url, username, credential);
            }
            _ => {
                return Err(CliError::InvalidConfig(
                    "TURN server requires both username and credential".to_string(),
                ));
            }
        }
    }

    Ok(config)
}

async fn create_host(config: SessionConfig, name: &str) -> Result<()> {
    let host = Participant::new_host(name.to_string())
        .map_err(|e| CliError::ParticipantCreation(e.to_string()))?;

    let lobby = Lobby::new("CLI Lobby".to_string(), host.clone())
        .map_err(|e| CliError::InvalidConfig(e.to_string()))?;

    let mut state = SessionState::new(host);
    state.set_lobby(lobby);

    let mut session = P2PSession::create_host_with_config(config)
        .await
        .map_err(|e| CliError::P2PConnection(e.to_string()))?;

    wait_for_peer_id(&mut session).await?;

    run_tui(&mut session, state).await
}

async fn join_session(config: SessionConfig, session_id_str: &str, name: &str) -> Result<()> {
    let guest = Participant::new_guest(name.to_string())
        .map_err(|e| CliError::ParticipantCreation(e.to_string()))?;

    let state = SessionState::new(guest);

    let session_id =
        SessionId::parse(session_id_str).map_err(|e| CliError::InvalidSessionId(e.to_string()))?;

    let mut session = P2PSession::join_with_config(config, session_id)
        .await
        .map_err(|e| CliError::P2PConnection(e.to_string()))?;

    wait_for_peer_id(&mut session).await?;

    run_tui(&mut session, state).await
}

async fn wait_for_peer_id(session: &mut P2PSession) -> Result<()> {
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    loop {
        session.poll_events(); // Process events to get peer ID

        if session.local_peer_id().is_some() {
            return Ok(());
        }

        if start.elapsed() > timeout {
            return Err(CliError::P2PConnection(
                "Timeout waiting for peer ID".to_string(),
            ));
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn run_tui(session: &mut P2PSession, state: SessionState) -> Result<()> {
    let mut terminal = tui::setup_terminal()?;

    let session_id = session.session_id().to_string();
    let local_peer_id = session.local_peer_id().map(|id| id.to_string());

    let mut app = App::new(state, session_id);

    if let Some(peer_id) = local_peer_id {
        app.set_local_peer_id(peer_id);
    }

    let result = run_app_loop(&mut terminal, &mut app, session).await;

    tui::restore_terminal(terminal)?;

    result
}
async fn run_app_loop(
    terminal: &mut tui::TuiTerminal,
    app: &mut App,
    session: &mut P2PSession,
) -> Result<()> {
    loop {
        terminal.draw(|f| tui::ui::render(f, app))?;

        match tui::event::read_events().await {
            Ok(AppEvent::Key(key)) => {
                app.handle_key(key);
                if app.should_quit {
                    break;
                }
            }
            Ok(AppEvent::Tick) => {
                // Poll P2P events
                let events = session.poll_events();

                for event in events {
                    app.handle_connection_event(&event);

                    match event {
                        ConnectionEvent::PeerConnected(peer_id) => {
                            handle_peer_connected(session, &mut app.session_state, peer_id).await?;
                        }
                        ConnectionEvent::PeerDisconnected(peer_id) => {
                            handle_peer_disconnected(session, &mut app.session_state, peer_id)
                                .await?;
                        }
                        ConnectionEvent::PeerTimedOut {
                            peer_id,
                            participant_id,
                            was_host,
                        } => {
                            handle_peer_timed_out(
                                session,
                                &mut app.session_state,
                                participant_id,
                                was_host,
                            )
                            .await?;
                        }
                        ConnectionEvent::MessageReceived { from, data } => {
                            handle_message_received(session, &mut app.session_state, from, data)
                                .await?;
                        }
                    }
                }
            }
            Err(_) => {}
        }
    }

    Ok(())
}
