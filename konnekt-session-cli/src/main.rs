use clap::{Parser, Subcommand};
use konnekt_session_cli::{CliError, Result};
use konnekt_session_core::Participant;
use konnekt_session_p2p::{ConnectionEvent, P2PSession, SessionConfig, SessionId};
use std::time::Duration;
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "konnekt-cli")]
#[command(
    version,
    about = "Konnekt Session CLI - P2P session management and testing"
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

        /// TURN server URL (optional, format: turn:host:port)
        #[arg(long)]
        turn_server: Option<String>,

        /// TURN username (required if turn-server is set)
        #[arg(long)]
        turn_username: Option<String>,

        /// TURN credential (required if turn-server is set)
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

        /// TURN server URL (optional, format: turn:host:port)
        #[arg(long)]
        turn_server: Option<String>,

        /// TURN username (required if turn-server is set)
        #[arg(long)]
        turn_username: Option<String>,

        /// TURN credential (required if turn-server is set)
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
                info!("Using TURN server: {}", turn_url);
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
    info!("Creating new session as host '{}'", name);
    info!(
        "Connecting to signalling server: {}",
        config.signalling_server
    );

    // Create participant
    let host = Participant::new_host(name.to_string())
        .map_err(|e| CliError::ParticipantCreation(e.to_string()))?;

    info!(
        "Host participant created: {} (ID: {})",
        host.name(),
        host.id()
    );

    // Create P2P session
    let mut session = P2PSession::create_host_with_config(config)
        .await
        .map_err(|e| CliError::P2PConnection(e.to_string()))?;

    info!("✓ Session created successfully!");
    info!("📋 Session ID: {}", session.session_id());

    // Wait for peer ID
    let local_id = wait_for_peer_id(&mut session).await?;
    info!("🔗 Local Peer ID: {}", local_id);

    info!("");
    info!("Share this command with guests to join:");
    info!(
        "  konnekt-cli join --server {} --session-id {}",
        "wss://match.konnektoren.help",
        session.session_id()
    );
    info!("");
    info!("Waiting for guests to connect...");
    info!("Press Ctrl+C to exit");
    info!("");

    // Event loop
    run_event_loop(&mut session, &host).await?;

    Ok(())
}

async fn join_session(config: SessionConfig, session_id_str: &str, name: &str) -> Result<()> {
    info!("Joining session as guest '{}'", name);
    info!("Session ID: {}", session_id_str);
    info!(
        "Connecting to signalling server: {}",
        config.signalling_server
    );

    // Create participant
    let guest = Participant::new_guest(name.to_string())
        .map_err(|e| CliError::ParticipantCreation(e.to_string()))?;

    info!(
        "Guest participant created: {} (ID: {})",
        guest.name(),
        guest.id()
    );

    // Parse session ID
    let session_id =
        SessionId::parse(session_id_str).map_err(|e| CliError::InvalidSessionId(e.to_string()))?;

    // Join P2P session
    let mut session = P2PSession::join_with_config(config, session_id)
        .await
        .map_err(|e| CliError::P2PConnection(e.to_string()))?;

    info!("✓ Joined session successfully!");

    // Wait for peer ID
    let local_id = wait_for_peer_id(&mut session).await?;
    info!("🔗 Local Peer ID: {}", local_id);

    info!("");
    info!("Waiting for connection to host...");
    info!("Press Ctrl+C to exit");
    info!("");

    // Event loop
    run_event_loop(&mut session, &guest).await?;

    Ok(())
}

async fn wait_for_peer_id(session: &mut P2PSession) -> Result<konnekt_session_p2p::PeerId> {
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    loop {
        if let Some(peer_id) = session.local_peer_id() {
            return Ok(peer_id);
        }

        if start.elapsed() > timeout {
            return Err(CliError::P2PConnection(
                "Timeout waiting for peer ID assignment".to_string(),
            ));
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn run_event_loop(session: &mut P2PSession, participant: &Participant) -> Result<()> {
    let mut interval = tokio::time::interval(Duration::from_millis(100));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Poll for events
                let events = session.poll_events();

                for event in events {
                    handle_event(session, participant, event).await?;
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("");
                info!("Shutting down...");
                break;
            }
        }
    }

    Ok(())
}

async fn handle_event(
    session: &mut P2PSession,
    participant: &Participant,
    event: ConnectionEvent,
) -> Result<()> {
    match event {
        ConnectionEvent::PeerConnected(peer_id) => {
            info!("🟢 Peer connected: {}", peer_id);

            // Send our participant info to the new peer
            let intro_msg = serde_json::json!({
                "type": "participant_info",
                "participant_id": participant.id().to_string(),
                "name": participant.name(),
                "role": format!("{}", participant.lobby_role()),
                "mode": format!("{}", participant.participation_mode()),
            });

            let data = serde_json::to_vec(&intro_msg)
                .map_err(|e| CliError::Serialization(e.to_string()))?;

            session
                .send_to(peer_id, data)
                .map_err(|e| CliError::MessageSend(e.to_string()))?;

            info!("📤 Sent participant info to peer {}", peer_id);
            info!("");
            info!("Connected peers: {}", session.connected_peers().len());
        }

        ConnectionEvent::PeerDisconnected(peer_id) => {
            warn!("🔴 Peer disconnected: {}", peer_id);
            info!("");
            info!("Connected peers: {}", session.connected_peers().len());
        }

        ConnectionEvent::MessageReceived { from, data } => {
            // Try to parse as JSON
            match serde_json::from_slice::<serde_json::Value>(&data) {
                Ok(msg) => {
                    if let Some(msg_type) = msg.get("type").and_then(|v| v.as_str()) {
                        match msg_type {
                            "participant_info" => {
                                let name = msg
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown");
                                let role = msg
                                    .get("role")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown");
                                let mode = msg
                                    .get("mode")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown");

                                info!("📥 Received participant info from peer {}:", from);
                                info!("   Name: {}", name);
                                info!("   Role: {}", role);
                                info!("   Mode: {}", mode);
                            }
                            _ => {
                                info!("📥 Received message from {}: {:?}", from, msg);
                            }
                        }
                    }
                }
                Err(_) => {
                    info!("📥 Received {} bytes from {}", data.len(), from);
                }
            }
        }
    }

    Ok(())
}
