//! Steps for `rating_match.feature`: the rating match protocol state machine
//! as seen by one player. Event names are the ones in the protocol doc
//! (`konnekt-session-workspace/protocols/rating-match.md`).

use cucumber::{given, then, when};
use konnekt_session_rating::protocol::{
    DynamicRatingMatch, MatchConfig, RatingMatchEvent as Event, RejoinTimedOut, RoundSigned,
};
use konnekt_session_tests::{RatingMatchUnderTest, SessionWorld};

fn protocol(world: &mut SessionWorld) -> &mut RatingMatchUnderTest {
    world
        .rating
        .protocol
        .as_mut()
        .expect("No rating match; use 'Given a rating match with N rounds'")
}

/// Builds an event from its protocol name. Payloads come from the caller's data.
fn event(name: &str, signed_rounds: u8) -> Event {
    let round = signed_rounds + 1;
    let events = [
        Event::SnapshotsVerified,
        Event::SnapshotRejected,
        Event::SnapshotTimeout,
        Event::RoundFinished,
        Event::RoundSigned(RoundSigned { round }),
        Event::RoundMismatch,
        Event::MatchSigned,
        Event::MatchMismatch,
        Event::OpponentLeft,
        Event::SignatureTimeout,
        Event::OpponentRejoined,
        Event::RejoinTimeout(RejoinTimedOut { signed_rounds }),
    ];
    events
        .into_iter()
        .find(|event| event.name() == name)
        .unwrap_or_else(|| panic!("Unknown rating match event '{name}'"))
}

/// Fires an event that must be accepted; keeps the caller's round counter in sync.
fn fire(world: &mut SessionWorld, name: &str) {
    let protocol = protocol(world);
    let event = event(name, protocol.signed_rounds);
    let signed = matches!(event, Event::RoundSigned(_));
    protocol
        .machine
        .handle(event)
        .unwrap_or_else(|error| panic!("'{name}' rejected: {error}"));
    if signed {
        protocol.signed_rounds += 1;
    }
}

#[given(expr = "a rating match with {int} round(s)")]
fn rating_match(world: &mut SessionWorld, rounds: u8) {
    world.rating.protocol = Some(RatingMatchUnderTest {
        machine: DynamicRatingMatch::new(MatchConfig { rounds }),
        signed_rounds: 0,
    });
}

#[given("both rating snapshots are verified")]
#[when("both rating snapshots are verified")]
fn snapshots_verified(world: &mut SessionWorld) {
    fire(world, "snapshots_verified");
}

#[given(expr = "{int} round(s) is/are played and co-signed")]
#[when(expr = "{int} round(s) is/are played and co-signed")]
fn rounds_co_signed(world: &mut SessionWorld, rounds: u8) {
    for _ in 0..rounds {
        fire(world, "round_finished");
        fire(world, "round_signed");
    }
}

#[given(expr = "{string} happens")]
#[when(expr = "{string} happens")]
fn happens(world: &mut SessionWorld, name: String) {
    fire(world, &name);
}

#[then(expr = "the rating match should be in state {string}")]
fn in_state(world: &mut SessionWorld, expected: String) {
    let state = protocol(world).machine.current_state();
    assert_eq!(format!("{state:?}"), expected);
}

#[then("the rating match should be finished")]
fn finished(world: &mut SessionWorld) {
    assert!(protocol(world).machine.is_finished());
}

#[then("the rating match should not be finished")]
fn not_finished(world: &mut SessionWorld) {
    assert!(!protocol(world).machine.is_finished());
}

#[then(expr = "{string} should be rejected")]
fn rejected(world: &mut SessionWorld, name: String) {
    let protocol = protocol(world);
    let before = protocol.machine.current_state();
    let event = event(&name, protocol.signed_rounds);
    assert!(
        protocol.machine.handle(event).is_err(),
        "'{name}' was accepted in {before:?}"
    );
    assert_eq!(protocol.machine.current_state(), before, "state changed");
}

#[then(expr = "{int} round(s) should be co-signed")]
fn co_signed(world: &mut SessionWorld, rounds: u8) {
    assert_eq!(protocol(world).signed_rounds, rounds);
}
