use cucumber::{given, then, when};
use konnekt_session_core::domain::{ActivityConfig, ActivityResult};
use konnekt_session_core::{ActivityRun, DomainCommand, DomainEvent, RunStatus};
use konnekt_session_rating::prelude::*;
use konnekt_session_tests::SessionWorld;

use super::player;
use super::rating_system_steps::rate;

/// Test game: higher score wins, equal scores draw.
struct HighScore;

impl RatedGame for HighScore {
    fn rating_key(&self) -> RatingKey {
        RatingKey::new("test.high-score.v1").expect("Valid key")
    }

    fn placement(&self, run: &ActivityRun) -> Option<Placement> {
        if run.status() != RunStatus::Completed {
            return None;
        }
        Placement::from_scores(
            run.results()
                .values()
                .map(|result| (result.participant_id, result.score.unwrap_or(0))),
        )
        .ok()
    }

    fn evidence(&self, run: &ActivityRun) -> Vec<u8> {
        run.id().as_bytes().to_vec()
    }
}

fn run(world: &SessionWorld) -> &ActivityRun {
    let run_id = world.rating.run_id.expect("No rated run");
    world.event_loop.get_run(&run_id).expect("Run not found")
}

// ===== Given Steps =====

#[given(expr = "guest {string} has joined")]
async fn guest_joined(world: &mut SessionWorld, name: String) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");
    let event = world
        .execute(DomainCommand::JoinLobby {
            lobby_id,
            guest_name: name.clone(),
        })
        .clone();
    match event {
        DomainEvent::GuestJoined { participant, .. } => {
            world.participant_ids.insert(name, participant.id());
        }
        event => panic!("Expected guest to join: {event:?}"),
    }
}

#[given(expr = "a rated {string} activity is in progress")]
async fn rated_activity_in_progress(world: &mut SessionWorld, name: String) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");
    let config = ActivityConfig::new("test-high-score-v1".into(), name, Default::default());
    world.execute(DomainCommand::QueueActivity { lobby_id, config });
    match world.execute(DomainCommand::StartNextRun { lobby_id }) {
        DomainEvent::RunStarted { run_id, .. } => world.rating.run_id = Some(*run_id),
        event => panic!("Expected run start: {event:?}"),
    }
}

// ===== When Steps =====

#[when(expr = "{string} submits score {int} for the rated activity")]
async fn submits_score(world: &mut SessionWorld, name: String, score: u32) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");
    let run_id = world.rating.run_id.expect("No rated run");
    let result = ActivityResult::new(run_id, player(world, &name)).with_score(score);
    world.execute(DomainCommand::SubmitResult {
        lobby_id,
        run_id,
        result,
    });
    assert!(!world.last_command_failed(), "{:?}", world.last_error);
}

#[when("the run is rated")]
async fn run_rated(world: &mut SessionWorld) {
    let game = HighScore;
    world.rating.placement = game.placement(run(world));
    rate(world, game.weight());
}

// ===== Then Steps =====

#[then(expr = "the rated game should place {string} before {string}")]
async fn places_before(world: &mut SessionWorld, first: String, second: String) {
    let placement = HighScore.placement(run(world)).expect("Run not rated");
    assert_eq!(placement.rank_of(&player(world, &first)), Some(0));
    assert_eq!(placement.rank_of(&player(world, &second)), Some(1));
}

#[then(expr = "the rated game should place {string} and {string} equal")]
async fn places_equal(world: &mut SessionWorld, a: String, b: String) {
    let placement = HighScore.placement(run(world)).expect("Run not rated");
    assert_eq!(placement.rank_of(&player(world, &a)), Some(0));
    assert_eq!(placement.rank_of(&player(world, &b)), Some(0));
}

#[then("the run should not be rated")]
async fn not_rated(world: &mut SessionWorld) {
    assert_eq!(HighScore.placement(run(world)), None);
}

#[then(expr = "the rating key should be {string}")]
async fn rating_key(_world: &mut SessionWorld, key: String) {
    assert_eq!(HighScore.rating_key().to_string(), key);
}
