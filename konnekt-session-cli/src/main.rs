use clap::{Parser, Subcommand};
use konnekt_session_cli::Result;
use konnekt_session_core::DomainCommand;
use konnekt_session_p2p::{ConnectionEvent, IceServer, P2PLoopBuilder, SessionId, SessionLoop};
use std::time::Duration;
use tracing::{debug, info, warn};

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

        /// Lobby name
        #[arg(short = 'l', long, default_value = "CLI Lobby")]
        lobby_name: String,

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
            lobby_name,
            name,
            turn_server,
            turn_username,
            turn_credential,
        } => {
            let ice_servers = build_ice_servers(turn_server, turn_username, turn_credential)?;
            create_host(&server, &lobby_name, &name, ice_servers).await?;
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
                info!("Using TURN server: {}", turn_url);
                ice_servers.push(IceServer::turn(turn_url, username, credential));
            }
            _ => {
                return Err(konnekt_session_cli::CliError::InvalidInput(
                    "TURN server requires both username and credential".to_string(),
                ));
            }
        }
    }

    Ok(ice_servers)
}

async fn create_host(
    server: &str,
    lobby_name: &str,
    host_name: &str,
    ice_servers: Vec<IceServer>,
) -> Result<()> {
    info!("Creating new session as host '{}'", host_name);

    // Build session using SessionLoop
    let (mut session_loop, session_id) = P2PLoopBuilder::new()
        .build_session_host(
            server,
            ice_servers.clone(),
            lobby_name.to_string(),
            host_name.to_string(),
        )
        .await?;

    let lobby_id = session_loop.lobby_id();

    info!("✅ Session created successfully!");
    info!("📋 Session ID: {}", session_id);
    info!("📋 Lobby ID: {}", lobby_id);
    info!("");
    info!("Share this command with guests to join:");
    info!(
        "  konnekt-cli join --server {} --session-id {}",
        server, session_id
    );
    info!("");
    info!("=== Session Active ===");
    info!("  Press Ctrl+C to quit");
    info!("");

    // Wait for peer ID to be assigned
    wait_for_peer_id(&mut session_loop).await?;

    run_event_loop(session_loop, true).await
}

async fn join_session(
    server: &str,
    session_id_str: &str,
    guest_name: &str,
    ice_servers: Vec<IceServer>,
) -> Result<()> {
    info!("Joining session as guest '{}'", guest_name);

    let session_id = SessionId::parse(session_id_str)?;

    // Build session using SessionLoop
    let (mut session_loop, lobby_id) = P2PLoopBuilder::new()
        .build_session_guest(server, session_id, ice_servers.clone())
        .await?;

    info!("✅ Connected to P2P network");
    info!("📋 Lobby ID: {}", lobby_id);

    // Wait for peer ID
    wait_for_peer_id(&mut session_loop).await?;

    // Wait for lobby to sync from host
    info!("⏳ Waiting for lobby sync...");
    wait_for_lobby_sync(&mut session_loop).await?;

    info!("✅ Lobby synced!");

    // Submit join command
    session_loop.submit_command(DomainCommand::JoinLobby {
        lobby_id,
        guest_name: guest_name.to_string(),
    })?;

    info!("");
    info!("=== Session Active ===");
    info!("  Press Ctrl+C to quit");
    info!("");

    run_event_loop(session_loop, false).await
}

/// Wait for peer ID to be assigned by Matchbox
async fn wait_for_peer_id(session_loop: &mut SessionLoop) -> Result<()> {
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        session_loop.poll();

        if session_loop.local_peer_id().is_some() {
            info!("✅ Peer ID assigned");
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    Err(konnekt_session_cli::CliError::InvalidInput(
        "Timeout waiting for peer ID".to_string(),
    ))
}

/// Wait for lobby to sync from host via P2P
async fn wait_for_lobby_sync(session_loop: &mut SessionLoop) -> Result<()> {
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    tracing::info!(
        "⏳ Waiting for lobby sync (up to {}s)...",
        timeout.as_secs()
    );

    while start.elapsed() < timeout {
        // Poll to process incoming messages
        let processed = session_loop.poll();

        if processed > 0 {
            tracing::debug!("Processed {} events during sync wait", processed);
        }

        // Check if we received lobby via P2P sync
        if let Some(lobby) = session_loop.get_lobby() {
            info!("✅ Lobby '{}' synced!", lobby.name());
            info!("   Host: {:?}", lobby.host_id());
            info!("   Participants: {}", lobby.participants().len());
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

    Err(konnekt_session_cli::CliError::InvalidInput(format!(
        "Timeout waiting for lobby {} to sync",
        session_loop.lobby_id()
    )))
}

/// Main event loop - PRESENTATION ONLY
/// All business logic is in SessionLoop (P2P + Core)
async fn run_event_loop(mut session_loop: SessionLoop, is_host: bool) -> Result<()> {
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    let mut last_participant_count = 0;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Poll session loop (auto-syncs P2P ↔ Core)
                let processed = session_loop.poll();

                if processed > 0 {
                    debug!("Processed {} events", processed);
                }

                // PRESENTATION: Display lobby state changes
                display_lobby_changes(&session_loop, &mut last_participant_count);

                // PRESENTATION: Display peer connections
                let peer_count = session_loop.connected_peers().len();
                debug!("Connected peers: {}", peer_count);
            }

            _ = tokio::signal::ctrl_c() => {
                info!("");
                info!("Received Ctrl+C, shutting down...");

                // Leave lobby gracefully if we're a guest
                if !is_host {
                    handle_graceful_shutdown(&mut session_loop).await?;
                }

                break;
            }
        }
    }

    info!("✅ Shutdown complete");
    Ok(())
}

/// Display lobby changes (presentation only)
fn display_lobby_changes(session_loop: &SessionLoop, last_count: &mut usize) {
    if let Some(lobby) = session_loop.get_lobby() {
        let current_count = lobby.participants().len();

        if current_count != *last_count {
            info!("👥 Participants: {}", current_count);

            for participant in lobby.participants().values() {
                let role = if participant.is_host() {
                    "Host"
                } else {
                    "Guest"
                };
                let mode = if participant.can_submit_results() {
                    "Active"
                } else {
                    "Spectating"
                };

                info!("  {} - {} ({})", participant.name(), role, mode);
            }

            *last_count = current_count;
        }
    }
}

/// Handle graceful shutdown for guests
async fn handle_graceful_shutdown(session_loop: &mut SessionLoop) -> Result<()> {
    if let Some(lobby) = session_loop.get_lobby() {
        // Find our participant ID (non-host)
        if let Some(participant) = lobby.participants().values().find(|p| !p.is_host()) {
            session_loop.submit_command(DomainCommand::LeaveLobby {
                lobby_id: session_loop.lobby_id(),
                participant_id: participant.id(),
            })?;

            // Give it a moment to send
            tokio::time::sleep(Duration::from_millis(200)).await;
            session_loop.poll();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_host_parsing() {
        let cli = Cli::parse_from(&[
            "konnekt-cli",
            "create-host",
            "--name",
            "Alice",
            "--lobby-name",
            "Test Lobby",
        ]);

        match cli.command {
            Commands::CreateHost {
                name, lobby_name, ..
            } => {
                assert_eq!(name, "Alice");
                assert_eq!(lobby_name, "Test Lobby");
            }
            _ => panic!("Expected CreateHost command"),
        }
    }

    #[test]
    fn test_join_parsing() {
        let session_id = "550e8400-e29b-41d4-a716-446655440000";
        let cli = Cli::parse_from(&[
            "konnekt-cli",
            "join",
            "--session-id",
            session_id,
            "--name",
            "Bob",
        ]);

        match cli.command {
            Commands::Join {
                session_id: sid,
                name,
                ..
            } => {
                assert_eq!(sid, session_id);
                assert_eq!(name, "Bob");
            }
            _ => panic!("Expected Join command"),
        }
    }

    #[test]
    fn test_turn_server_validation() {
        // TURN server without credentials should fail
        let result = build_ice_servers(Some("turn:turn.example.com:3478".to_string()), None, None);

        assert!(result.is_err());

        // TURN server with credentials should succeed
        let result = build_ice_servers(
            Some("turn:turn.example.com:3478".to_string()),
            Some("user".to_string()),
            Some("pass".to_string()),
        );

        assert!(result.is_ok());
    }
}
