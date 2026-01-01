use konnekt_session_core::DomainCommand;
use konnekt_session_p2p::{IceServer, SessionId, SessionLoopV2Builder};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Get session ID from command line
    let session_id_str = std::env::args()
        .nth(1)
        .expect("Usage: cargo run --example v2_basic_guest <session_id>");

    let session_id = SessionId::parse(&session_id_str)?;
    let signalling_server = "wss://match.konnektoren.help";
    let ice_servers = IceServer::default_stun_servers();

    // ✅ NEW API: Clean and simple
    let (mut session_loop, lobby_id) = SessionLoopV2Builder::new()
        .build_guest(signalling_server, session_id.clone(), ice_servers)
        .await?;

    println!("✅ Joined session: {}", session_id);
    println!("📋 Lobby ID: {}", lobby_id);

    // Wait for lobby sync
    println!("⏳ Waiting for lobby sync...");
    for i in 0..100 {
        session_loop.poll();

        if session_loop.get_lobby().is_some() {
            println!("✅ Lobby synced!");
            break;
        }

        if i == 99 {
            eprintln!("❌ Timeout: Lobby never synced");
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Submit join command
    println!("📤 Joining lobby as 'Guest'...");
    session_loop.submit_command(DomainCommand::JoinLobby {
        lobby_id,
        guest_name: "Guest".to_string(),
    })?;

    // Main event loop
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    let mut tick_count = 0;

    loop {
        interval.tick().await;

        let processed = session_loop.poll();

        if processed > 0 {
            println!("📡 Processed {} events", processed);
        }

        tick_count += 1;
        if tick_count % 50 == 0 {
            if let Some(lobby) = session_loop.get_lobby() {
                println!(
                    "🏠 Lobby '{}' has {} participants",
                    lobby.name(),
                    lobby.participants().len()
                );
            }
        }
    }
}
