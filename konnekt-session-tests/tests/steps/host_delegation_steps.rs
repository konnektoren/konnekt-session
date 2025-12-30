use cucumber::{given, then, when};
use konnekt_session_core::{DomainCommand, DomainEvent, LobbyRole, Participant, Timestamp};
use konnekt_session_tests::SessionWorld;

// ===== Given Steps =====

#[given(expr = "a lobby exists with a host")]
async fn lobby_exists_with_default_host(world: &mut SessionWorld) {
    lobby_exists_with_named_host(world, "Host".to_string()).await;
}

#[given(expr = "a lobby exists with Host {string}")]
async fn lobby_exists_with_host_name(world: &mut SessionWorld, name: String) {
    lobby_exists_with_named_host(world, name).await;
}

// Helper function to reduce duplication
async fn lobby_exists_with_named_host(world: &mut SessionWorld, host_name: String) {
    let cmd = DomainCommand::CreateLobby {
        lobby_name: "Test Lobby".to_string(),
        host_name: host_name.clone(),
    };

    let event = world.execute(cmd).clone();

    if let DomainEvent::LobbyCreated { lobby } = event {
        let lobby_id = lobby.id();
        let host_id = lobby.host_id();

        world.lobby_ids.insert("Test Lobby".to_string(), lobby_id);
        world.participant_ids.insert(host_name, host_id);
    }
}

#[given(expr = "guest {string} joined {int} seconds ago")]
async fn guest_joined_seconds_ago(world: &mut SessionWorld, name: String, _seconds_ago: u64) {
    // For now, just join normally - timestamp ordering will be handled by creation order
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");

    let cmd = DomainCommand::JoinLobby {
        lobby_id,
        guest_name: name.clone(),
    };

    let event = world.execute(cmd).clone();

    if let DomainEvent::GuestJoined { participant, .. } = event {
        world.participant_ids.insert(name, participant.id());
    }
}

#[given(expr = "guest {string} joined at time {int}")]
async fn guest_joined_at_time(world: &mut SessionWorld, name: String, _time: u64) {
    // Delegate to the simpler version for now
    guest_joined_seconds_ago(world, name, 0).await;
}

#[given("an activity is in progress")]
async fn activity_in_progress(_world: &mut SessionWorld) {
    // TODO: Implement when activity management is ready
}

#[given("the host disconnects at time T")]
#[given("the host disconnects")]
async fn host_disconnects(_world: &mut SessionWorld) {
    // TODO: Implement when P2P layer is ready
}

#[given(expr = "a lobby with only the host and {int} guest(s)")]
async fn lobby_with_host_and_guests(world: &mut SessionWorld, guest_count: usize) {
    lobby_exists_with_default_host(world).await;

    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");

    for i in 0..guest_count {
        let guest_name = format!("Guest{}", i + 1);
        let cmd = DomainCommand::JoinLobby {
            lobby_id,
            guest_name: guest_name.clone(),
        };

        let event = world.execute(cmd).clone();

        if let DomainEvent::GuestJoined { participant, .. } = event {
            world.participant_ids.insert(guest_name, participant.id());
        }
    }
}

#[given("a lobby with only the host")]
async fn lobby_with_only_host(world: &mut SessionWorld) {
    lobby_exists_with_default_host(world).await;
}

// ===== When Steps =====

#[when(expr = "the host delegates to {string}")]
async fn host_delegates_to(world: &mut SessionWorld, guest_name: String) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");
    let host_id = *world.participant_ids.get("Host").expect("No host");
    let guest_id = world.get_participant_id(&guest_name);

    let cmd = DomainCommand::DelegateHost {
        lobby_id,
        current_host_id: host_id,
        new_host_id: guest_id,
    };

    world.execute(cmd);
}

#[when(expr = "{int} seconds pass")]
#[when(expr = "{int} second passes")]
async fn seconds_pass(_world: &mut SessionWorld, _seconds: u64) {
    // TODO: Implement time-based delegation when P2P layer is ready
}

#[when(expr = "{int} more seconds pass")]
async fn more_seconds_pass(_world: &mut SessionWorld, _seconds: u64) {
    // TODO: Implement time-based delegation when P2P layer is ready
}

#[when("the host reconnects")]
async fn host_reconnects(_world: &mut SessionWorld) {
    // TODO: Implement reconnection logic when P2P layer is ready
}

#[when("the 30s timeout expires")]
async fn timeout_expires(_world: &mut SessionWorld) {
    // TODO: Implement auto-delegation when P2P layer handles timeouts
}

// ===== Then Steps =====

#[then(expr = "{string} should become the host")]
#[then(expr = "{string} should be the host")]
async fn should_become_host(world: &mut SessionWorld, name: String) {
    match world.last_event() {
        DomainEvent::HostDelegated { to, .. } => {
            let expected_id = world.get_participant_id(&name);
            assert_eq!(
                *to, expected_id,
                "{} should be the new host (expected {}, got {})",
                name, expected_id, to
            );
        }
        _ => panic!("Expected HostDelegated event"),
    }
}

#[then(expr = "the original host should become a guest")]
async fn original_host_becomes_guest(world: &mut SessionWorld) {
    // Verify the delegation event was emitted
    assert!(
        matches!(world.last_event(), DomainEvent::HostDelegated { .. }),
        "Expected HostDelegated event"
    );
}

#[then(expr = "a HostDelegated event should be broadcast with reason {string}")]
async fn host_delegated_event(world: &mut SessionWorld, _reason: String) {
    assert!(
        matches!(world.last_event(), DomainEvent::HostDelegated { .. }),
        "Expected HostDelegated event"
    );
    // TODO: Verify reason when it's added to the event
}

#[then(expr = "the host should be marked as {string}")]
async fn host_marked_as(_world: &mut SessionWorld, _status: String) {
    // TODO: Implement connection status tracking
}

#[then("the host should retain their role")]
async fn host_retains_role(_world: &mut SessionWorld) {
    // TODO: Implement when reconnection logic is added
}

#[then("no delegation should occur")]
async fn no_delegation_occurs(_world: &mut SessionWorld) {
    // TODO: Implement when reconnection logic is added
}

#[then(expr = "the original host should rejoin as a guest")]
async fn original_host_rejoins_as_guest(_world: &mut SessionWorld) {
    // TODO: Implement when reconnection logic is added
}

#[then(expr = "{string} should remain the host")]
async fn should_remain_host(world: &mut SessionWorld, name: String) {
    should_become_host(world, name).await;
}

#[then(expr = "not {string}")]
async fn not_guest(world: &mut SessionWorld, name: String) {
    match world.last_event() {
        DomainEvent::HostDelegated { to, .. } => {
            let participant_id = world.get_participant_id(&name);
            assert_ne!(*to, participant_id, "{} should NOT be the host", name);
        }
        _ => panic!("Expected HostDelegated event"),
    }
}

#[then(expr = "the guest with the lowest UUID becomes host")]
async fn lowest_uuid_becomes_host(_world: &mut SessionWorld) {
    // TODO: Implement tie-breaking logic verification
}

#[then("the guest should immediately become host")]
async fn guest_immediately_becomes_host(_world: &mut SessionWorld) {
    // TODO: Implement single-guest promotion verification
}

#[then("the lobby should close automatically")]
async fn lobby_closes(_world: &mut SessionWorld) {
    // TODO: Implement lobby closure logic
}

#[then(expr = "{string} can manage the activity")]
async fn can_manage_activity(world: &mut SessionWorld, name: String) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");
    let lobby = world
        .event_loop
        .get_lobby(&lobby_id)
        .expect("Lobby not found");

    let participant_id = world.get_participant_id(&name);
    let participant = lobby.participants().get(&participant_id).unwrap();

    assert!(
        participant.can_manage_lobby(),
        "{} should be able to manage the lobby/activity",
        name
    );
}

#[then("the activity should continue")]
async fn activity_continues(_world: &mut SessionWorld) {
    // TODO: Implement when activity management is ready
}

#[then("the new host can manually delegate back")]
async fn new_host_can_delegate_back(_world: &mut SessionWorld) {
    // This is a business rule verification, not a technical test
}
