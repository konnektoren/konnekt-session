mod support;

use konnekt_session_core::{domain::ActivityConfig, DomainCommand};
use support::SessionFixture;

#[test]
fn test_guest_joins_and_syncs_lobby() {
    let mut fixture = SessionFixture::new(1);

    fixture.tick(10);

    let guest_lobby = fixture.guests[0].get_lobby();
    assert!(guest_lobby.is_some(), "Guest should have synced lobby");

    let lobby = guest_lobby.unwrap();
    assert_eq!(lobby.name(), "Test Lobby");
    assert_eq!(lobby.participants().len(), 1);

    fixture.guests[0]
        .submit_command(DomainCommand::JoinLobby {
            lobby_id: fixture.lobby_id,
            guest_name: "Guest1".to_string(),
        })
        .unwrap();

    fixture.tick(10);

    let host_lobby = fixture.host.get_lobby().unwrap();
    let guest_lobby = fixture.guests[0].get_lobby().unwrap();

    assert_eq!(host_lobby.participants().len(), 2);
    assert_eq!(guest_lobby.participants().len(), 2);
}

#[test]
fn test_multiple_guests() {
    let mut fixture = SessionFixture::new(3);

    fixture.tick(500);

    for (i, guest) in fixture.guests.iter_mut().enumerate() {
        let cmd = DomainCommand::JoinLobby {
            lobby_id: fixture.lobby_id,
            guest_name: format!("Guest{}", i + 1),
        };
        guest.submit_command(cmd).unwrap();
    }

    fixture.tick(900);

    let host_count = fixture
        .host
        .get_lobby()
        .expect("Host should have lobby")
        .participants()
        .len();
    assert_eq!(host_count, 4, "Host should see 4 participants");

    for (index, guest) in fixture.guests.iter().enumerate() {
        let guest_count = guest
            .get_lobby()
            .unwrap_or_else(|| panic!("Guest {} should have lobby", index + 1))
            .participants()
            .len();
        assert_eq!(guest_count, 4, "Guest {} should see 4 participants", index + 1);
    }
}

#[test]
fn test_activity_queue_and_start_run() {
    let mut fixture = SessionFixture::new(1);

    fixture.tick(10);

    fixture.guests[0]
        .submit_command(DomainCommand::JoinLobby {
            lobby_id: fixture.lobby_id,
            guest_name: "Guest1".to_string(),
        })
        .unwrap();

    fixture.tick(10);

    // Host queues activity
    let config = ActivityConfig::new(
        "echo-challenge-v1".to_string(),
        "Echo Test".to_string(),
        serde_json::json!({}),
    );

    fixture
        .host
        .submit_command(DomainCommand::QueueActivity {
            lobby_id: fixture.lobby_id,
            config,
        })
        .unwrap();

    fixture.tick(10);

    // Both should see queued activity
    assert_eq!(fixture.host.get_lobby().unwrap().activity_queue().len(), 1);

    // Host starts next run
    fixture
        .host
        .submit_command(DomainCommand::StartNextRun {
            lobby_id: fixture.lobby_id,
        })
        .unwrap();

    fixture.tick(10);

    // Host should have active run, queue should be empty
    assert!(fixture.host.get_lobby().unwrap().has_active_run());
    assert!(fixture.host.get_lobby().unwrap().activity_queue().is_empty());
}

#[test]
fn test_activity_completion() {
    let mut fixture = SessionFixture::new(1);

    fixture.tick(200);

    fixture.guests[0]
        .submit_command(DomainCommand::JoinLobby {
            lobby_id: fixture.lobby_id,
            guest_name: "Guest1".to_string(),
        })
        .unwrap();

    fixture.tick(250);

    // IDs must come from HOST's lobby — the host assigns UUIDs.
    // Guests receive echoed JoinLobby commands and create different UUIDs locally.
    let host_lobby = fixture.host.get_lobby().unwrap();
    let host_participant_id = host_lobby.participants().values()
        .find(|p| p.is_host())
        .expect("Host participant")
        .id();
    let guest_participant_id = host_lobby.participants().values()
        .find(|p| !p.is_host())
        .expect("Guest participant on host")
        .id();

    // Queue and start
    let config = ActivityConfig::new(
        "echo-challenge-v1".to_string(),
        "Echo Test".to_string(),
        serde_json::json!({}),
    );

    fixture.host.submit_command(DomainCommand::QueueActivity {
        lobby_id: fixture.lobby_id,
        config,
    }).unwrap();

    fixture.tick(250);

    fixture.host.submit_command(DomainCommand::StartNextRun {
        lobby_id: fixture.lobby_id,
    }).unwrap();

    fixture.tick(250);

    let run_id = fixture.host
        .get_lobby()
        .unwrap()
        .active_run_id()
        .expect("Run should be active");

    // Host submits result
    fixture.host.submit_command(DomainCommand::SubmitResult {
        lobby_id: fixture.lobby_id,
        run_id,
        result: konnekt_session_core::domain::ActivityResult::new(run_id, host_participant_id)
            .with_score(100),
    }).unwrap();

    fixture.tick(250);

    // Guest submits result
    fixture.guests[0].submit_command(DomainCommand::SubmitResult {
        lobby_id: fixture.lobby_id,
        run_id,
        result: konnekt_session_core::domain::ActivityResult::new(run_id, guest_participant_id)
            .with_score(80),
    }).unwrap();

    fixture.tick(400);

    // Run should be completed on host
    // After completion, active_run is cleared from lobby
    assert!(!fixture.host.get_lobby().unwrap().has_active_run());
}
