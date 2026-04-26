mod support;

use support::SessionFixture;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn init_test_tracing() {
    let _ = tracing_subscriber::registry()
        .with(EnvFilter::new("debug"))
        .with(fmt::layer().with_test_writer())
        .try_init();
}

#[test]
fn test_host_creation_with_tracing() {
    init_test_tracing();
    let mut fixture = SessionFixture::new(0);
    fixture.tick(1);

    let lobby = fixture.host.get_lobby().expect("Lobby should exist");
    assert_eq!(lobby.name(), "Test Lobby");
    assert_eq!(lobby.participants().len(), 1);
    tracing::info!("✅ Session created: {}", fixture.lobby_id);
}
