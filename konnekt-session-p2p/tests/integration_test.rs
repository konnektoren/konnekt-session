use konnekt_session_core::DomainCommand;
use konnekt_session_p2p::{IceServer, P2PLoopBuilder, SessionId};

#[tokio::test]
#[ignore] // Ignore by default (requires network)
async fn test_host_creates_lobby() {
    tracing_subscriber::fmt::init();

    let signalling_server = "wss://match.konnektoren.help";
    let ice_servers = IceServer::default_stun_servers();

    // Create host session
    let (mut session_loop, session_id) = P2PLoopBuilder::new()
        .build_session_host(
            signalling_server,
            ice_servers,
            "Test Lobby".to_string(),
            "Host".to_string(),
        )
        .await
        .expect("Failed to create host session");

    println!("✅ Host session created: {}", session_id);

    // Poll once to process lobby creation
    session_loop.poll();

    // Verify lobby exists
    let lobby = session_loop.get_lobby().expect("Lobby should exist");
    assert_eq!(lobby.name(), "Test Lobby");
    assert_eq!(lobby.participants().len(), 1);

    println!("✅ Lobby verified");
}

#[tokio::test]
#[ignore] // Requires running host first
async fn test_guest_joins_lobby() {
    // This test requires a running host session
    // In practice, you'd coordinate this with a separate test harness

    let signalling_server = "wss://match.konnektoren.help";
    let session_id = SessionId::parse("PASTE_SESSION_ID_HERE").unwrap();
    let ice_servers = IceServer::default_stun_servers();

    let (mut session_loop, lobby_id) = P2PLoopBuilder::new()
        .build_session_guest(signalling_server, session_id, ice_servers)
        .await
        .expect("Failed to join session");

    println!("✅ Guest joined session, lobby: {}", lobby_id);

    // Submit join command
    session_loop
        .submit_command(DomainCommand::JoinLobby {
            lobby_id,
            guest_name: "Alice".to_string(),
        })
        .expect("Failed to submit join command");

    // Poll to process
    session_loop.poll();

    println!("✅ Join command processed");
}
