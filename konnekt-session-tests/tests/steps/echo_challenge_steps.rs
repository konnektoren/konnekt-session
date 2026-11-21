use cucumber::{given, then, when};
use konnekt_session_core::domain::{ActivityConfig, ActivityResult, RunStatus};
use konnekt_session_core::{DomainCommand, DomainEvent, EchoChallenge, EchoResult};
use konnekt_session_tests::SessionWorld;

// ===== Given Steps =====

#[given(expr = "an Echo Challenge with prompt {string} is planned")]
async fn echo_challenge_planned(world: &mut SessionWorld, prompt: String) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");

    let challenge = EchoChallenge::new(prompt.clone());
    let metadata = ActivityConfig::new(
        EchoChallenge::activity_type().to_string(),
        format!("Echo: {}", prompt),
        challenge.to_config(),
    );
    let activity_id = metadata.id;

    let cmd = DomainCommand::QueueActivity {
        lobby_id,
        config: metadata,
    };

    world.execute(cmd);

    // Store activity ID for later reference
    world
        .lobby_ids
        .insert(format!("Activity:{}", prompt), activity_id);
}

#[given(expr = "an Echo Challenge with prompt {string} is in progress")]
async fn echo_challenge_in_progress(world: &mut SessionWorld, prompt: String) {
    echo_challenge_planned(world, prompt.clone()).await;

    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");
    let run_id = match world.execute(DomainCommand::StartNextRun { lobby_id }) {
        DomainEvent::RunStarted { run_id, .. } => *run_id,
        event => panic!("Expected run start: {event:?}"),
    };
    world.lobby_ids.insert(format!("Run:{}", prompt), run_id);
}

#[given(expr = "an Echo Challenge with prompt {string} is completed")]
async fn echo_challenge_completed(world: &mut SessionWorld, prompt: String) {
    echo_challenge_in_progress(world, prompt.clone()).await;

    let run_id = *world
        .lobby_ids
        .get(&format!("Run:{}", prompt))
        .expect("No run");
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");
    let host_id = *world.participant_ids.get("Host").expect("No host");

    // Submit result to complete
    let result = EchoResult::new(prompt, 1000);
    let cmd = DomainCommand::SubmitResult {
        lobby_id,
        run_id,
        result: ActivityResult::new(run_id, host_id)
            .with_data(result.to_json())
            .with_score(100)
            .with_time(1000),
    };

    world.execute(cmd);
}

#[given(expr = "an Echo Challenge with prompt {string} and time limit {int}ms is planned")]
async fn echo_challenge_with_time_limit(world: &mut SessionWorld, prompt: String, time_limit: u64) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");

    let challenge = EchoChallenge::new(prompt.clone()).with_time_limit(time_limit);
    let metadata = ActivityConfig::new(
        EchoChallenge::activity_type().to_string(),
        format!("Echo: {}", prompt),
        challenge.to_config(),
    );
    let activity_id = metadata.id;

    let cmd = DomainCommand::QueueActivity {
        lobby_id,
        config: metadata,
    };

    world.execute(cmd);
    world
        .lobby_ids
        .insert(format!("Activity:{}", prompt), activity_id);
}

#[given(expr = "an Echo Challenge with prompt {string}")]
async fn echo_challenge_exists(world: &mut SessionWorld, prompt: String) {
    let challenge = EchoChallenge::new(prompt);
    // Store in world for later assertions
    world.last_error = Some(serde_json::to_string(&challenge).unwrap());
}

// ===== When Steps =====

#[when(expr = "the host plans an Echo Challenge with prompt {string}")]
async fn plan_echo_challenge(world: &mut SessionWorld, prompt: String) {
    echo_challenge_planned(world, prompt).await;
}

#[given(expr = r#"{string} has submitted response {string}"#)]
#[when(expr = r#"{string} submits response {string}"#)]
async fn submit_response(world: &mut SessionWorld, participant_name: String, response: String) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");
    let participant_id = world.get_participant_id(&participant_name);

    // Find the current in-progress activity
    let lobby = world
        .event_loop
        .get_lobby(&lobby_id)
        .expect("Lobby not found");
    let run_id = lobby.active_run_id().expect("No activity in progress");
    let run = world.event_loop.get_run(&run_id).expect("No run");

    // Deserialize the challenge to calculate score
    let challenge = EchoChallenge::from_config(run.config().config.clone()).unwrap();
    let score = challenge.calculate_score(&response);

    let result = EchoResult::new(response, 1000);

    let cmd = DomainCommand::SubmitResult {
        lobby_id,
        run_id,
        result: ActivityResult::new(run_id, participant_id)
            .with_data(result.to_json())
            .with_score(score)
            .with_time(1000),
    };

    world.execute(cmd);
}

#[when(expr = r#"{string} submits response {string} \(score {int}\)"#)]
async fn submit_response_with_score(
    world: &mut SessionWorld,
    participant_name: String,
    response: String,
    _expected_score: u32,
) {
    submit_response(world, participant_name, response).await;
}

#[when(expr = r#"{string} submits response {string} after {int} milliseconds"#)]
async fn submit_response_with_time(
    world: &mut SessionWorld,
    participant_name: String,
    response: String,
    time_ms: u64,
) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");
    let participant_id = world.get_participant_id(&participant_name);

    let lobby = world
        .event_loop
        .get_lobby(&lobby_id)
        .expect("Lobby not found");
    let run_id = lobby.active_run_id().expect("No activity in progress");
    let run = world.event_loop.get_run(&run_id).expect("No run");

    let challenge = EchoChallenge::from_config(run.config().config.clone()).unwrap();
    let score = challenge.calculate_score(&response);

    let result = EchoResult::new(response, time_ms);

    let cmd = DomainCommand::SubmitResult {
        lobby_id,
        run_id,
        result: ActivityResult::new(run_id, participant_id)
            .with_data(result.to_json())
            .with_score(score)
            .with_time(time_ms),
    };

    world.execute(cmd);
}

#[when(expr = r#"{string} tries to submit response {string}"#)]
async fn try_submit_response(world: &mut SessionWorld, participant_name: String, response: String) {
    submit_response(world, participant_name, response).await;
}

#[when("the activity config is serialized to JSON")]
async fn serialize_config(_world: &mut SessionWorld) {
    // Challenge already stored in world.last_error as JSON
}

#[when("deserialized back to an Echo Challenge")]
async fn deserialize_config(world: &mut SessionWorld) {
    let json = world.last_error.as_ref().expect("No serialized data");
    let challenge: EchoChallenge = serde_json::from_str(json).unwrap();
    // Store back for assertions
    world.last_error = Some(serde_json::to_string(&challenge).unwrap());
}

// ===== Then Steps =====

#[then(expr = "the activity type should be {string}")]
async fn activity_type_is(world: &mut SessionWorld, expected_type: String) {
    match world.last_event() {
        DomainEvent::ActivityQueued {
            config: metadata, ..
        } => {
            assert_eq!(metadata.activity_type, expected_type);
        }
        _ => panic!("Expected ActivityPlanned event"),
    }
}

#[then(expr = "the activity name should be {string}")]
async fn activity_name_is(world: &mut SessionWorld, expected_name: String) {
    match world.last_event() {
        DomainEvent::ActivityQueued {
            config: metadata, ..
        } => {
            assert_eq!(metadata.name, expected_name);
        }
        _ => panic!("Expected ActivityPlanned event"),
    }
}

#[then(expr = r#"{string} should receive score {int}"#)]
async fn participant_receives_score(
    world: &mut SessionWorld,
    participant_name: String,
    expected_score: u32,
) {
    let participant_id = world.get_participant_id(&participant_name);
    let result = match world.last_event() {
        DomainEvent::ResultSubmitted { result, .. } => result,
        DomainEvent::RunEnded { results, .. } => results
            .iter()
            .find(|r| r.participant_id == participant_id)
            .expect("Missing participant result"),
        event => panic!("Expected result event: {event:?}"),
    };
    assert_eq!(result.participant_id, participant_id);
    assert_eq!(result.score, Some(expected_score));
}

#[then("the result should be recorded")]
async fn result_recorded(world: &mut SessionWorld) {
    assert!(matches!(
        world.last_event(),
        DomainEvent::ResultSubmitted { .. }
            | DomainEvent::RunEnded {
                status: RunStatus::Completed,
                ..
            }
    ));
}

#[then("the result should be accepted")]
async fn result_accepted(world: &mut SessionWorld) {
    assert!(matches!(
        world.last_event(),
        DomainEvent::ResultSubmitted { .. }
            | DomainEvent::RunEnded {
                status: RunStatus::Completed,
                ..
            }
    ));
}

#[then(expr = "the result should record time {int} milliseconds")]
async fn result_records_time(world: &mut SessionWorld, expected_time: u64) {
    match world.last_event() {
        DomainEvent::ResultSubmitted { result, .. } => {
            assert_eq!(result.time_taken_ms, Some(expected_time));
        }
        DomainEvent::RunEnded { results, .. } => assert!(
            results
                .iter()
                .any(|r| r.time_taken_ms == Some(expected_time))
        ),
        _ => panic!("Expected ResultSubmitted event"),
    }
}

#[then(expr = r#"results from {string} should be preserved"#)]
async fn results_preserved(world: &mut SessionWorld, activity_prompt: String) {
    let run_id = *world
        .lobby_ids
        .get(&format!("Run:{}", activity_prompt))
        .expect("No run");
    let results = world.event_loop.get_run(&run_id).expect("No run").results();
    assert!(
        !results.is_empty(),
        "Results from '{}' were not preserved",
        activity_prompt
    );
}

#[then("the new activity should complete")]
async fn new_activity_completes(world: &mut SessionWorld) {
    assert!(matches!(
        world.last_event(),
        DomainEvent::RunEnded {
            status: RunStatus::Completed,
            ..
        }
    ));
}

#[then(expr = r#"the prompt should be {string}"#)]
async fn prompt_is(world: &mut SessionWorld, expected_prompt: String) {
    let json = world.last_error.as_ref().expect("No challenge data");
    let challenge: EchoChallenge = serde_json::from_str(json).unwrap();
    assert_eq!(challenge.prompt, expected_prompt);
}

#[then("the activity should be created")]
async fn activity_created(world: &mut SessionWorld) {
    assert!(matches!(
        world.last_event(),
        DomainEvent::ActivityQueued { .. }
    ));
}

#[when("the host cancels the current Echo Challenge")]
async fn cancel_echo_challenge(world: &mut SessionWorld) {
    let lobby_id = *world.lobby_ids.get("Test Lobby").expect("No lobby");
    let run_id = world
        .event_loop
        .get_lobby(&lobby_id)
        .expect("No lobby")
        .active_run_id()
        .expect("No active run");
    world.execute(DomainCommand::CancelRun { lobby_id, run_id });
}

#[then("the Echo Challenge is cancelled with Alice's result")]
async fn echo_cancelled_with_result(world: &mut SessionWorld) {
    let alice = world.get_participant_id("Alice");
    match world.last_event() {
        DomainEvent::RunEnded {
            status: RunStatus::Cancelled,
            results,
            ..
        } => {
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].participant_id, alice);
        }
        event => panic!("Expected cancelled run: {event:?}"),
    }
}
