use cucumber::{given, then, when};
use konnekt_session_rating::prelude::*;
use konnekt_session_tests::{RatingSystemKind, SessionWorld};

use super::{player, register};

/// Gives a player a new rating in both systems.
fn rate_new(world: &mut SessionWorld, name: &str) {
    register(world, name);
    world
        .rating
        .glicko2
        .insert(name.into(), Glicko2System::default().initial());
    world
        .rating
        .elo
        .insert(name.into(), EloSystem::default().initial());
}

/// Rates the current placement with every player that has a rating.
pub(crate) fn rate(world: &mut SessionWorld, weight: f64) {
    let placement = world.rating.placement.clone().expect("No placement");
    world.rating.before = world
        .rating
        .players
        .keys()
        .filter_map(|name| Some((name.clone(), world.rating.value(name)?)))
        .collect();
    let result = match world.rating.system {
        RatingSystemKind::Glicko2 => {
            let ratings = ratings(world, &world.rating.glicko2);
            Glicko2System::default()
                .rate(&ratings, &placement, weight)
                .map(|rated| store(world, rated, |world| &mut world.rating.glicko2))
        }
        RatingSystemKind::Elo => {
            let ratings = ratings(world, &world.rating.elo);
            EloSystem::default()
                .rate(&ratings, &placement, weight)
                .map(|rated| store(world, rated, |world| &mut world.rating.elo))
        }
    };
    if let Err(error) = result {
        world.rating.error = Some(error.to_string());
    }
}

fn ratings<R: Clone>(
    world: &SessionWorld,
    by_name: &std::collections::HashMap<String, R>,
) -> Vec<(ParticipantId, R)> {
    let mut ratings: Vec<_> = by_name
        .iter()
        .map(|(name, rating)| (player(world, name), rating.clone()))
        .collect();
    ratings.sort_by_key(|(id, _)| *id);
    ratings
}

fn store<R>(
    world: &mut SessionWorld,
    rated: Vec<(ParticipantId, R)>,
    target: impl Fn(&mut SessionWorld) -> &mut std::collections::HashMap<String, R>,
) {
    for (id, rating) in rated {
        let name = world
            .rating
            .players
            .iter()
            .find(|(_, player)| **player == id)
            .map(|(name, _)| name.clone())
            .expect("Rated player has a name");
        target(world).insert(name, rating);
    }
}

// ===== Given Steps =====

#[given(expr = "rated players {string} and {string}")]
async fn rated_players(world: &mut SessionWorld, a: String, b: String) {
    rate_new(world, &a);
    rate_new(world, &b);
}

#[given(expr = "rated players {string}, {string} and {string}")]
async fn three_rated_players(world: &mut SessionWorld, a: String, b: String, c: String) {
    rate_new(world, &a);
    rate_new(world, &b);
    rate_new(world, &c);
}

#[given(expr = "player {string} without a rating")]
async fn unrated_player(world: &mut SessionWorld, name: String) {
    register(world, &name);
}

#[given("the Elo rating system")]
async fn elo_system(world: &mut SessionWorld) {
    world.rating.system = RatingSystemKind::Elo;
}

#[given(expr = "{string} has a rating of {float}")]
async fn has_rating(world: &mut SessionWorld, name: String, rating: f64) {
    match world.rating.system {
        RatingSystemKind::Glicko2 => {
            let current = world.rating.glicko2.entry(name).or_default();
            current.rating = rating;
        }
        RatingSystemKind::Elo => {
            world.rating.elo.entry(name).or_default().rating = rating;
        }
    }
}

// ===== When Steps =====

#[when("the match is rated")]
async fn match_rated(world: &mut SessionWorld) {
    rate(world, 1.0);
}

#[when(expr = "the match is rated with weight {float}")]
async fn match_rated_with_weight(world: &mut SessionWorld, weight: f64) {
    rate(world, weight);
}

// ===== Then Steps =====

#[then(expr = "{string} should have a rating of {float}")]
async fn should_have_rating(world: &mut SessionWorld, name: String, expected: f64) {
    let value = world.rating.value(&name).expect("No rating");
    assert_eq!(value.round(), expected, "Rating of {name} is {value}");
}

#[then(expr = "the rating of {string} should have increased")]
async fn increased(world: &mut SessionWorld, name: String) {
    let (before, after) = change(world, &name);
    assert!(after > before, "{name}: {before} -> {after}");
}

#[then(expr = "the rating of {string} should have decreased")]
async fn decreased(world: &mut SessionWorld, name: String) {
    let (before, after) = change(world, &name);
    assert!(after < before, "{name}: {before} -> {after}");
}

fn change(world: &SessionWorld, name: &str) -> (f64, f64) {
    assert!(world.rating.error.is_none(), "{:?}", world.rating.error);
    let before = *world.rating.before.get(name).expect("Not rated before");
    (before, world.rating.value(name).expect("No rating"))
}

#[then(expr = "rating should be rejected with {string}")]
async fn rating_rejected(world: &mut SessionWorld, message: String) {
    let error = world.rating.error.as_deref().expect("Rating was accepted");
    assert!(error.contains(&message), "Unexpected error: {error}");
}
