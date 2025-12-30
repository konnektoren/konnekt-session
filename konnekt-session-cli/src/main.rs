use clap::{Parser, Subcommand};
use konnekt_session_cli::{
    application::RuntimeBuilder,
    infrastructure::{CliError, Result},
};
use konnekt_session_core::DomainCommand;
use konnekt_session_p2p::{ConnectionEvent, IceServer, SessionId};
use std::time::Duration;
use tracing::info;

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
                info!("Using TURN server: {}", turn_url);
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
    info!("Creating new session as host '{}'", name);

    // Build runtime - returns session_id and lobby_id directly
    let (mut runtime, session_id, lobby_id) = RuntimeBuilder::new()
        .domain_batch_size(10)
        .build_host(server, ice_servers.clone())
        .await?;

    info!("📋 Session ID: {}", session_id);
    info!("📋 Lobby ID: {}", lobby_id);

    // Wait for peer ID to be assigned
    wait_for_peer_id(&mut runtime).await?;

    // Create lobby in domain
    let host_id = {
        let cmd = DomainCommand::CreateLobby {
            lobby_id: Some(lobby_id), // Use the session ID as lobby ID
            lobby_name: "CLI Lobby".to_string(),
            host_name: name.to_string(),
        };

        runtime.domain_loop_mut().submit(cmd)?;
        runtime.domain_loop_mut().poll();

        let events = runtime.domain_loop_mut().drain_events();
        match events.first() {
            Some(konnekt_session_core::DomainEvent::LobbyCreated { lobby }) => {
                info!("✓ Lobby created: {}", lobby.name());
                lobby.host_id() // 🆕 Return host_id from this block
            }
            _ => {
                return Err(CliError::InvalidConfig(
                    "Failed to create lobby".to_string(),
                ));
            }
        }
    };

    // Set lobby context in runtime
    runtime.set_lobby(lobby_id, true);

    // 🆕 Broadcast initial LobbyCreated event to any peers (now we have host_id)
    runtime
        .p2p_loop_mut()
        .broadcast_event(konnekt_session_p2p::DomainEvent::LobbyCreated {
            lobby_id,
            host_id, // 🆕 Use the host_id we got from the block above
            name: "CLI Lobby".to_string(),
        })?;

    info!("📤 Initial LobbyCreated broadcast");

    info!("");
    info!("✓ Session created successfully!");
    info!("📋 Session ID: {}", session_id);
    info!("");
    info!("Share this command with guests to join:");
    info!(
        "  konnekt-cli join --server {} --session-id {}",
        server, session_id
    );
    info!("");
    info!("💡 Your participation mode: Active");
    info!("");
    info!("=== Interactive Session ===");
    info!("  Press Ctrl+C to quit");
    info!("");

    run_event_loop(runtime).await
}

async fn join_session(
    server: &str,
    session_id_str: &str,
    name: &str,
    ice_servers: Vec<IceServer>,
) -> Result<()> {
    info!("Joining session as guest '{}'", name);

    let session_id = SessionId::parse(session_id_str)?;

    // Build runtime - returns lobby_id directly
    let (mut runtime, _session_id, lobby_id) = RuntimeBuilder::new()
        .domain_batch_size(10)
        .build_guest(server, session_id, ice_servers.clone())
        .await?;

    info!("✓ Connected to P2P network");

    // Wait for peer ID
    wait_for_peer_id(&mut runtime).await?;

    // Wait for lobby to sync from host
    info!("⏳ Waiting for lobby sync...");
    wait_for_lobby_sync(&mut runtime, lobby_id).await?;

    info!("✓ Lobby synced!");

    // Now lobby is available, we can continue
    runtime.set_lobby(lobby_id, false);

    info!("");
    info!("💡 Your participation mode: Active");
    info!("");
    info!("=== Interactive Session ===");
    info!("  Press Ctrl+C to quit");
    info!("");

    run_event_loop(runtime).await
}

/// Wait for peer ID to be assigned by Matchbox
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

/// Wait for lobby to sync from host via P2P
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
            info!("✅ Lobby '{}' synced!", lobby.name());
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err(CliError::P2PConnection(format!(
        "Timeout waiting for lobby {} to sync",
        lobby_id
    )))
}

/// Handle a single connection event
fn handle_connection_event(
    event: &ConnectionEvent,
    runtime: &mut konnekt_session_cli::DualLoopRuntime,
) {
    match event {
        ConnectionEvent::PeerConnected(peer_id) => {
            info!("🟢 Peer connected: {}", peer_id);

            // Send lobby state if we're host
            runtime.handle_peer_connected(*peer_id);
        }
        ConnectionEvent::PeerDisconnected(peer_id) => {
            info!("🔴 Peer disconnected: {} (grace period started)", peer_id);
        }
        ConnectionEvent::PeerTimedOut {
            peer_id, was_host, ..
        } => {
            info!("⏰ Peer timed out: {} (was_host: {})", peer_id, was_host);
        }
        ConnectionEvent::MessageReceived { from, data } => {
            info!("📥 Received {} bytes from {}", data.len(), from);
        }
    }
}

/// Main event loop - processes P2P and domain events
async fn run_event_loop(mut runtime: konnekt_session_cli::DualLoopRuntime) -> Result<()> {
    let mut interval = tokio::time::interval(Duration::from_millis(100));

    // Setup Ctrl+C handler
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for Ctrl+C");
    };

    tokio::pin!(ctrl_c);

    info!("Press Ctrl+C to quit...");

    loop {
        tokio::select! {
            biased;

            _ = &mut ctrl_c => {
                info!("");
                info!("🛑 Received Ctrl+C, shutting down...");
                break;
            }

            _ = interval.tick() => {
                // Tick the runtime
                let stats = runtime.tick();

                // Handle connection events
                let events: Vec<_> = runtime.drain_connection_events();
                for event in &events {
                    handle_connection_event(event, &mut runtime);
                }

                // Sleep if idle
                if stats.total() == 0 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        }
    }

    info!("✓ Shutdown complete");
    Ok(())
}
