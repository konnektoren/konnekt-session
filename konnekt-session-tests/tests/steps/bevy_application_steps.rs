use cucumber::{given, then, when};
use konnekt_session_core::{DomainCommand, DomainEvent};
use konnekt_session_tests::SessionWorld;
use uuid::Uuid;

#[given("a Bevy session app is initialized")]
async fn bevy_session_app_initialized(world: &mut SessionWorld) {
    world.init_bevy();
}

#[when(expr = "the host submits CreateLobby for {string}")]
async fn host_submits_create_lobby(world: &mut SessionWorld, lobby_name: String) {
    let lobby_id = Uuid::new_v4();
    world.lobby_ids.insert(lobby_name.clone(), lobby_id);

    world.bevy_submit(DomainCommand::CreateLobby {
        lobby_id: Some(lobby_id),
        lobby_name,
        host_name: "Host".to_string(),
    });
}

#[when(expr = "a guest named {string} submits JoinLobby")]
async fn guest_submits_join_lobby(world: &mut SessionWorld, guest_name: String) {
    let lobby_id = *world
        .lobby_ids
        .values()
        .next()
        .expect("No lobby id set in world");

    world.bevy_submit(DomainCommand::JoinLobby {
        lobby_id,
        guest_name,
    });
}

#[when(expr = "the Bevy app ticks {int} time")]
async fn bevy_app_ticks(world: &mut SessionWorld, ticks: usize) {
    world.bevy_tick(ticks);
}

#[then(expr = "the Bevy event log should contain {int} event")]
async fn bevy_event_log_should_contain(world: &mut SessionWorld, expected: usize) {
    let log = world.bevy_event_log();
    assert_eq!(
        log.len(),
        expected,
        "expected {expected} event(s), got {}",
        log.len()
    );
}

#[then(expr = "event {int} should be {string} with sequence {int}")]
async fn event_should_be_with_sequence(
    world: &mut SessionWorld,
    index_1based: usize,
    event_name: String,
    expected_sequence: u64,
) {
    let log = world.bevy_event_log();
    let event = log
        .get(index_1based - 1)
        .unwrap_or_else(|| panic!("event #{index_1based} does not exist"));

    assert_eq!(
        event.sequence, expected_sequence,
        "event #{index_1based} sequence mismatch"
    );

    match (&event.event, event_name.as_str()) {
        (DomainEvent::LobbyCreated { .. }, "LobbyCreated") => {}
        (DomainEvent::GuestJoined { .. }, "GuestJoined") => {}
        (other, expected) => panic!("expected {expected}, got {other:?}"),
    }
}

#[then(expr = "the lobby {string} should have {int} participants in Bevy domain")]
async fn lobby_should_have_participants_in_bevy(
    world: &mut SessionWorld,
    lobby_name: String,
    expected_count: usize,
) {
    let lobby_id = *world
        .lobby_ids
        .get(&lobby_name)
        .unwrap_or_else(|| panic!("Unknown lobby '{lobby_name}'"));

    let count = world.bevy_lobby_participant_count(lobby_id);
    assert_eq!(
        count, expected_count,
        "lobby '{lobby_name}' expected {expected_count} participant(s), got {count}"
    );
}
