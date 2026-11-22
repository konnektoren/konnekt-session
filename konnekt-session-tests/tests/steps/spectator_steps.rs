use cucumber::{given, then, when};
use konnekt_session_core::{
    ActivityConfig, DomainCommand, DomainEvent, Lobby, Participant, ParticipationMode,
};
use konnekt_session_p2p::application::host_failover::failover_candidates;
use konnekt_session_tests::SessionWorld;
use uuid::Uuid;

fn lobby(world: &SessionWorld) -> &Lobby {
    world.get_lobby("Test Lobby").expect("No lobby")
}

fn join(world: &mut SessionWorld, name: String, mode: ParticipationMode) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");
    let participant = Participant::new_guest(name.clone())
        .expect("valid guest name")
        .with_participation_mode(mode);
    let event = world
        .execute(DomainCommand::AddParticipant {
            lobby_id,
            participant,
        })
        .clone();
    match event {
        DomainEvent::GuestJoined { participant, .. } => {
            world.participant_ids.insert(name, participant.id());
        }
        event => panic!("Expected GuestJoined, got {event:?}"),
    }
}

#[given(expr = "guest {string} joined as a spectator")]
#[when(expr = "guest {string} joins as a spectator")]
async fn joins_as_spectator(world: &mut SessionWorld, name: String) {
    join(world, name, ParticipationMode::Spectating);
}

#[given(expr = "guest {string} joined as a player")]
#[when(expr = "guest {string} joins as a player")]
async fn joins_as_player(world: &mut SessionWorld, name: String) {
    join(world, name, ParticipationMode::Active);
}

#[when("the host starts an activity")]
async fn host_starts_activity(world: &mut SessionWorld) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");
    world.execute(DomainCommand::QueueActivity {
        lobby_id,
        config: ActivityConfig::new("quiz".into(), "Quiz".into(), serde_json::Value::Null),
    });
    match world.execute(DomainCommand::StartNextRun { lobby_id }) {
        DomainEvent::RunStarted { .. } => {}
        event => panic!("Expected RunStarted, got {event:?}"),
    }
}

#[when("the host role is assigned automatically")]
async fn host_assigned_automatically(world: &mut SessionWorld) {
    let mut copy = lobby(world).clone();
    world.elected_host = Some(copy.auto_delegate_host().map_err(|error| error.to_string()));
}

#[then("the delegation should be rejected")]
async fn delegation_rejected(world: &mut SessionWorld) {
    assert!(
        world.last_command_failed(),
        "Expected delegation to fail, got {:?}",
        world.last_event()
    );
    let host = world.get_participant_id("Host");
    assert_eq!(lobby(world).host_id(), host, "host must not change");
}

#[then(expr = "{string} should be in Spectating mode")]
async fn is_spectating(world: &mut SessionWorld, name: String) {
    let id = world.get_participant_id(&name);
    assert_eq!(
        lobby(world).participants()[&id].participation_mode(),
        ParticipationMode::Spectating
    );
}

#[then(expr = "{string} should be in Active mode")]
async fn is_active(world: &mut SessionWorld, name: String) {
    let id = world.get_participant_id(&name);
    assert_eq!(
        lobby(world).participants()[&id].participation_mode(),
        ParticipationMode::Active
    );
}

#[then(expr = "{string} should not be the host")]
async fn is_not_host(world: &mut SessionWorld, name: String) {
    let id = world.get_participant_id(&name);
    assert!(!lobby(world).participants()[&id].is_host());
    assert_ne!(lobby(world).host_id(), id);
}

fn required_submitter(world: &SessionWorld, name: &str) -> bool {
    let id = world.get_participant_id(name);
    let run_id = lobby(world).active_run_id().expect("No active run");
    world
        .event_loop
        .get_run(&run_id)
        .expect("Run not found")
        .required_submitters()
        .contains(&id)
}

#[then(expr = "{string} should be a required submitter")]
async fn is_required_submitter(world: &mut SessionWorld, name: String) {
    assert!(required_submitter(world, &name));
}

#[then(expr = "{string} should not be a required submitter")]
async fn is_not_required_submitter(world: &mut SessionWorld, name: String) {
    assert!(!required_submitter(world, &name));
}

#[then(expr = "the automatic host should be {string}")]
async fn automatic_host_is(world: &mut SessionWorld, name: String) {
    let expected = world.get_participant_id(&name);
    match &world.elected_host {
        Some(Ok(id)) => assert_eq!(*id, expected),
        other => panic!("Expected {name} to be elected, got {other:?}"),
    }
}

#[then("no automatic host should be found")]
async fn no_automatic_host(world: &mut SessionWorld) {
    assert!(
        matches!(world.elected_host, Some(Err(_))),
        "Expected no host, got {:?}",
        world.elected_host
    );
}

#[then(expr = "the failover candidates should be {string}")]
async fn failover_candidates_are(world: &mut SessionWorld, names: String) {
    let expected: Vec<Uuid> = names
        .split(',')
        .map(|name| world.get_participant_id(name.trim()))
        .collect();
    assert_eq!(failover_candidates(lobby(world)), expected);
}
