mod support;

use konnekt_session_core::DomainCommand;
use support::SessionFixture;

#[test]
fn test_guest_joins_and_syncs_lobby() {
    let mut fixture = SessionFixture::new(1);

    // Tick to allow snapshot sync
    fixture.tick(10);

    // Guest should have synced lobby
    let guest_lobby = fixture.guests[0].get_lobby();
    assert!(guest_lobby.is_some(), "Guest should have synced lobby");

    let lobby = guest_lobby.unwrap();
    assert_eq!(lobby.name(), "Test Lobby");
    assert_eq!(lobby.participants().len(), 1); // Just host so far

    // Guest joins
    fixture.guests[0]
        .submit_command(DomainCommand::JoinLobby {
            lobby_id: fixture.lobby_id,
            guest_name: "Guest1".to_string(),
        })
        .unwrap();

    // Tick to process join
    fixture.tick(10);

    // Both host and guest should see 2 participants
    let host_lobby = fixture.host.get_lobby().unwrap();
    let guest_lobby = fixture.guests[0].get_lobby().unwrap();

    assert_eq!(host_lobby.participants().len(), 2);
    assert_eq!(guest_lobby.participants().len(), 2);
}

#[test]
fn test_multiple_guests() {
    let mut fixture = SessionFixture::new(3);

    // Wait for initial snapshot sync
    println!("🔄 Waiting for snapshot sync...");
    fixture.poll_until_stable(50);

    println!("📊 After snapshot sync:");
    fixture.print_state();

    // All guests join
    println!("\n📤 Submitting join commands...");
    for (i, guest) in fixture.guests.iter_mut().enumerate() {
        let cmd = DomainCommand::JoinLobby {
            lobby_id: fixture.lobby_id,
            guest_name: format!("Guest{}", i + 1),
        };
        guest.submit_command(cmd).unwrap();
    }

    // Wait for joins to propagate
    println!("\n🔄 Waiting for joins to propagate...");
    fixture.poll_until_stable(50);

    println!("\n📊 Final state:");
    fixture.print_state();

    // Assert consistent state
    fixture.assert_consistent_state(4); // host + 3 guests
}

#[test]
fn test_activity_plan_and_start() {
    let mut fixture = SessionFixture::new(1);

    // Sync
    fixture.tick(10);

    // Guest joins
    fixture.guests[0]
        .submit_command(DomainCommand::JoinLobby {
            lobby_id: fixture.lobby_id,
            guest_name: "Guest1".to_string(),
        })
        .unwrap();

    fixture.tick(10);

    // Host plans activity
    let challenge = konnekt_session_core::EchoChallenge::new("Test".to_string());
    let metadata = konnekt_session_core::domain::ActivityMetadata::new(
        "echo-challenge-v1".to_string(),
        "Echo Test".to_string(),
        challenge.to_config(),
    );

    fixture
        .host
        .submit_command(DomainCommand::PlanActivity {
            lobby_id: fixture.lobby_id,
            metadata,
        })
        .unwrap();

    fixture.tick(10);

    // Both should see planned activity
    assert_eq!(fixture.host.get_lobby().unwrap().activities().len(), 1);
    assert_eq!(fixture.guests[0].get_lobby().unwrap().activities().len(), 1);

    // Host starts activity
    let activity_id = fixture.host.get_lobby().unwrap().activities()[0].id;

    fixture
        .host
        .submit_command(DomainCommand::StartActivity {
            lobby_id: fixture.lobby_id,
            activity_id,
        })
        .unwrap();

    fixture.tick(10);

    // Both should see activity in progress
    let host_current = fixture.host.get_lobby().unwrap().current_activity();
    let guest_current = fixture.guests[0].get_lobby().unwrap().current_activity();

    assert!(host_current.is_some());
    assert!(guest_current.is_some());
    assert_eq!(host_current.unwrap().id, activity_id);
    assert_eq!(guest_current.unwrap().id, activity_id);
}

#[test]
fn test_activity_completion() {
    let mut fixture = SessionFixture::new(1);

    // Setup: join + plan + start
    fixture.poll_until_stable(20);

    fixture.guests[0]
        .submit_command(DomainCommand::JoinLobby {
            lobby_id: fixture.lobby_id,
            guest_name: "Guest1".to_string(),
        })
        .unwrap();

    fixture.poll_until_stable(20);

    // Now get participant IDs (after stabilization)
    let host_lobby = fixture.host.get_lobby().unwrap();
    let guest_lobby = fixture.guests[0].get_lobby().unwrap();

    let host_participant_id = host_lobby
        .participants()
        .values()
        .find(|p| p.is_host())
        .expect("Host should have a host participant")
        .id();

    let guest_participant_id = guest_lobby
        .participants()
        .values()
        .find(|p| !p.is_host())
        .expect("Guest should have a guest participant")
        .id();

    // Plan activity
    let challenge = konnekt_session_core::EchoChallenge::new("Test".to_string());
    let metadata = konnekt_session_core::domain::ActivityMetadata::new(
        "echo-challenge-v1".to_string(),
        "Echo Test".to_string(),
        challenge.to_config(),
    );

    fixture
        .host
        .submit_command(DomainCommand::PlanActivity {
            lobby_id: fixture.lobby_id,
            metadata,
        })
        .unwrap();

    fixture.poll_until_stable(20);

    let activity_id = fixture.host.get_lobby().unwrap().activities()[0].id;

    // Start activity
    fixture
        .host
        .submit_command(DomainCommand::StartActivity {
            lobby_id: fixture.lobby_id,
            activity_id,
        })
        .unwrap();

    fixture.poll_until_stable(20);

    // Submit results
    let result = konnekt_session_core::EchoResult::new("Test".to_string(), 100);

    fixture
        .host
        .submit_command(DomainCommand::SubmitResult {
            lobby_id: fixture.lobby_id,
            result: konnekt_session_core::domain::ActivityResult::new(
                activity_id,
                host_participant_id,
            )
            .with_data(result.to_json())
            .with_score(100),
        })
        .unwrap();

    fixture.poll_until_stable(20);

    fixture.guests[0]
        .submit_command(DomainCommand::SubmitResult {
            lobby_id: fixture.lobby_id,
            result: konnekt_session_core::domain::ActivityResult::new(
                activity_id,
                guest_participant_id,
            )
            .with_data(result.to_json())
            .with_score(100),
        })
        .unwrap();

    // ✅ Wait for completion to propagate
    fixture.poll_until_stable(30);

    // Assert activity completed
    let host_activity = &fixture.host.get_lobby().unwrap().activities()[0];
    let guest_activity = &fixture.guests[0].get_lobby().unwrap().activities()[0];

    assert_eq!(
        host_activity.status,
        konnekt_session_core::domain::ActivityStatus::Completed,
        "Host should see activity as completed"
    );
    assert_eq!(
        guest_activity.status,
        konnekt_session_core::domain::ActivityStatus::Completed,
        "Guest should see activity as completed"
    );

    // Both should see 2 results
    let host_results = fixture.host.get_lobby().unwrap().get_results(activity_id);
    let guest_results = fixture.guests[0]
        .get_lobby()
        .unwrap()
        .get_results(activity_id);

    assert_eq!(host_results.len(), 2, "Host should see 2 results");
    assert_eq!(guest_results.len(), 2, "Guest should see 2 results");
}
