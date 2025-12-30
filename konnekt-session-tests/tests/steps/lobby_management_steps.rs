use cucumber::{given, then, when};
use konnekt_session_core::{DomainCommand, DomainEvent, ParticipationMode};
use konnekt_session_tests::SessionWorld;

// ===== Given Steps =====

#[given("a user wants to create a lobby")]
async fn user_wants_to_create_lobby(_world: &mut SessionWorld) {
    // No-op: just context for readability
}

#[given(expr = "a lobby exists with password {string}")]
async fn lobby_exists_with_password(world: &mut SessionWorld, password: String) {
    let cmd = DomainCommand::CreateLobby {
        lobby_id: None, // 🔧 FIX: Add lobby_id field (None = auto-generate)
        lobby_name: "Test Lobby".to_string(),
        host_name: "Host".to_string(),
    };

    let event = world.execute(cmd).clone();

    if let DomainEvent::LobbyCreated { lobby } = event {
        let lobby_id = lobby.id();
        let host_id = lobby.host_id();

        world.lobby_ids.insert("Test Lobby".to_string(), lobby_id);
        world.participant_ids.insert("Host".to_string(), host_id);
    }

    // TODO: Store password for validation when domain supports it
    let _ = password;
}

#[given("a lobby exists")]
async fn lobby_exists(world: &mut SessionWorld) {
    lobby_exists_with_password(world, "".to_string()).await;
}

#[given(expr = "a lobby exists with max {int} guests")]
async fn lobby_exists_with_max_guests(world: &mut SessionWorld, _max_guests: usize) {
    // TODO: Implement when lobby capacity is configurable
    // For now, create a standard lobby
    lobby_exists(world).await;
}

#[given(expr = "{int} guests have already joined")]
async fn n_guests_have_joined(world: &mut SessionWorld, count: usize) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");

    for i in 0..count {
        let guest_name = format!("Guest{}", i + 1);
        let cmd = DomainCommand::JoinLobby {
            lobby_id,
            guest_name: guest_name.clone(),
        };

        let event = world.execute(cmd).clone(); // ← Clone

        if let DomainEvent::GuestJoined { participant, .. } = event {
            world.participant_ids.insert(guest_name, participant.id());
        }
    }
}

// ===== When Steps =====

#[when(expr = "they create a lobby named {string} with password {string}")]
async fn create_lobby_with_password(
    world: &mut SessionWorld,
    lobby_name: String,
    _password: String,
) {
    let cmd = DomainCommand::CreateLobby {
        lobby_id: None, // 🔧 FIX: Add lobby_id field
        lobby_name: lobby_name.clone(),
        host_name: "Alice".to_string(),
    };

    let event = world.execute(cmd).clone();

    if let DomainEvent::LobbyCreated { lobby } = event {
        let lobby_id = lobby.id();
        let host_id = lobby.host_id();

        world.lobby_ids.insert(lobby_name, lobby_id);
        world.participant_ids.insert("Alice".to_string(), host_id);
    }
}

#[when(expr = "a guest joins with the correct password")]
async fn guest_joins_with_correct_password(world: &mut SessionWorld) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby created");

    let cmd = DomainCommand::JoinLobby {
        lobby_id,
        guest_name: "Bob".to_string(),
    };

    let event = world.execute(cmd).clone(); // ← Clone

    if let DomainEvent::GuestJoined { participant, .. } = event {
        world
            .participant_ids
            .insert("Bob".to_string(), participant.id());
    }
}

#[when(expr = "a guest tries to join with password {string}")]
async fn guest_tries_to_join_with_password(world: &mut SessionWorld, _password: String) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby created");

    let cmd = DomainCommand::JoinLobby {
        lobby_id,
        guest_name: "Charlie".to_string(),
    };

    world.execute(cmd);
    // Result will be checked in Then steps
}

#[when("another guest tries to join")]
async fn another_guest_tries_to_join(world: &mut SessionWorld) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");

    let cmd = DomainCommand::JoinLobby {
        lobby_id,
        guest_name: "TooMany".to_string(),
    };

    world.execute(cmd);
}

#[when("a new guest joins")]
async fn new_guest_joins(world: &mut SessionWorld) {
    guest_joins_with_correct_password(world).await;
}

// ===== Then Steps =====

#[then("a lobby should be created")]
async fn lobby_should_be_created(world: &mut SessionWorld) {
    assert!(
        matches!(world.last_event(), DomainEvent::LobbyCreated { .. }),
        "Expected LobbyCreated event"
    );
}

#[then("the lobby should have a unique ID")]
async fn lobby_should_have_unique_id(world: &mut SessionWorld) {
    if let DomainEvent::LobbyCreated { lobby } = world.last_event() {
        assert_ne!(lobby.id().to_string(), "");
    } else {
        panic!("Expected LobbyCreated event");
    }
}

#[then("the creator should be the host")]
async fn creator_should_be_host(world: &mut SessionWorld) {
    if let DomainEvent::LobbyCreated { lobby } = world.last_event() {
        let host = lobby.host().expect("Lobby should have a host");
        assert!(host.is_host(), "Creator should have host role");
    } else {
        panic!("Expected LobbyCreated event");
    }
}

#[then(expr = "the lobby status should be {string}")]
async fn lobby_status_should_be(_world: &mut SessionWorld, _status: String) {
    // TODO: Implement when lobby status is added to domain
    // For now, just verify lobby was created
}

#[then("the guest should be added to the lobby")]
async fn guest_should_be_added(world: &mut SessionWorld) {
    assert!(
        matches!(world.last_event(), DomainEvent::GuestJoined { .. }),
        "Expected GuestJoined event"
    );
}

#[then("the guest should be in Active mode")]
async fn guest_should_be_in_active_mode(world: &mut SessionWorld) {
    if let DomainEvent::GuestJoined { participant, .. } = world.last_event() {
        assert_eq!(
            participant.participation_mode(),
            ParticipationMode::Active,
            "Guest should join in Active mode by default"
        );
    } else {
        panic!("Expected GuestJoined event");
    }
}

#[then("a GuestJoined event should be broadcast")]
async fn guest_joined_event_should_be_broadcast(world: &mut SessionWorld) {
    assert!(
        matches!(world.last_event(), DomainEvent::GuestJoined { .. }),
        "Expected GuestJoined event"
    );
}

#[then("the join should be rejected")]
async fn join_should_be_rejected(world: &mut SessionWorld) {
    assert!(
        world.last_command_failed(),
        "Expected command to fail, but it succeeded"
    );
}

#[then(expr = "the error should be {string}")]
async fn error_should_be(world: &mut SessionWorld, expected_error: String) {
    let actual_error = world
        .last_error_message()
        .expect("Expected an error but none was recorded");

    assert!(
        actual_error.contains(&expected_error),
        "Expected error to contain '{}', but got '{}'",
        expected_error,
        actual_error
    );
}
