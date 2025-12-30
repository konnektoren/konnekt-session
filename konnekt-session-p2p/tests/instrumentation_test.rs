use konnekt_session_p2p::{IceServer, P2PLoopBuilder, SessionId};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn init_test_tracing() {
    let _ = tracing_subscriber::registry()
        .with(EnvFilter::new("debug"))
        .with(fmt::layer().with_test_writer())
        .try_init();
}

#[tokio::test]
#[ignore] // Requires network
async fn test_host_creation_with_tracing() {
    init_test_tracing();

    let signalling_server = "wss://match.konnektoren.help";
    let ice_servers = IceServer::default_stun_servers();

    let result = P2PLoopBuilder::new()
        .build_session_host(
            signalling_server,
            ice_servers,
            "Test Lobby".to_string(),
            "Host".to_string(),
        )
        .await;

    assert!(result.is_ok());

    let (session_loop, session_id) = result.unwrap();
    assert!(session_loop.get_lobby().is_some());
    tracing::info!("✅ Session created: {}", session_id);
}
