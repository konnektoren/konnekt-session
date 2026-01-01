use konnekt_session_core::DomainCommand;
use konnekt_session_p2p::{IceServer, SessionLoopV2Builder};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let signalling_server = "wss://match.konnektoren.help";
    let ice_servers = IceServer::default_stun_servers();

    // ✅ NEW API: Clean and simple
    let (mut session_loop, session_id) = SessionLoopV2Builder::new()
        .build_host(
            signalling_server,
            ice_servers,
            "My Lobby".to_string(),
            "Host".to_string(),
        )
        .await?;

    println!("✅ Session created: {}", session_id);
    println!("📋 Share this session ID with guests!");

    // Main event loop
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    let mut tick_count = 0;

    loop {
        interval.tick().await;

        // Poll P2P + Domain
        let processed = session_loop.poll();

        if processed > 0 {
            println!("📡 Processed {} events", processed);
        }

        // Every 5 seconds, print lobby status
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

        // Example: Submit a command
        if tick_count == 100 {
            if let Some(lobby) = session_loop.get_lobby() {
                println!("📝 Testing command submission...");
                session_loop.submit_command(DomainCommand::PlanActivity {
                    lobby_id: lobby.id(),
                    metadata: konnekt_session_core::domain::ActivityMetadata::new(
                        "test".to_string(),
                        "Test Activity".to_string(),
                        serde_json::json!({}),
                    ),
                })?;
            }
        }
    }
}
