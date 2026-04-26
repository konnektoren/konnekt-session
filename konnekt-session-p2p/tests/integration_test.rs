mod support;

use konnekt_session_core::DomainCommand;
use support::SessionFixture;

#[test]
fn test_host_creates_lobby() {
    let mut fixture = SessionFixture::new(0);
    fixture.tick(1);
    let lobby = fixture.host.get_lobby().expect("Lobby should exist");
    assert_eq!(lobby.name(), "Test Lobby");
    assert_eq!(lobby.participants().len(), 1);
}

#[test]
fn test_guest_joins_lobby() {
    let mut fixture = SessionFixture::new(1);
    fixture.tick(10);

    fixture.guests[0]
        .submit_command(DomainCommand::JoinLobby {
            lobby_id: fixture.lobby_id,
            guest_name: "Alice".to_string(),
        })
        .expect("Failed to submit join command");

    fixture.tick(10);

    let host_lobby = fixture.host.get_lobby().expect("Host lobby should exist");
    let guest_lobby = fixture.guests[0]
        .get_lobby()
        .expect("Guest lobby should exist");
    assert_eq!(host_lobby.participants().len(), 2);
    assert_eq!(guest_lobby.participants().len(), 2);
}
