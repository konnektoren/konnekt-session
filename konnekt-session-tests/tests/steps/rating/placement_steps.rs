use cucumber::gherkin::Step;
use cucumber::{then, when};
use konnekt_session_rating::Placement;
use konnekt_session_tests::SessionWorld;

use super::player;

fn set(world: &mut SessionWorld, placement: konnekt_session_rating::Result<Placement>) {
    match placement {
        Ok(placement) => world.rating.placement = Some(placement),
        Err(error) => world.rating.error = Some(error.to_string()),
    }
}

// ===== When Steps =====

#[when(expr = "{string} wins against {string}")]
async fn wins_against(world: &mut SessionWorld, winner: String, loser: String) {
    let placement = Placement::win(player(world, &winner), player(world, &loser));
    set(world, placement);
}

#[when(expr = "{string} and {string} draw")]
async fn draw(world: &mut SessionWorld, a: String, b: String) {
    let placement = Placement::draw(vec![player(world, &a), player(world, &b)]);
    set(world, placement);
}

#[when("the placement is made from the scores:")]
async fn from_scores(world: &mut SessionWorld, step: &Step) {
    let table = step.table.as_ref().expect("Scores table");
    let scores: Vec<_> = table
        .rows
        .iter()
        .skip(1)
        .map(|row| {
            let score = row[1].parse::<u32>().expect("Score must be a number");
            (player(world, &row[0]), score)
        })
        .collect();
    set(world, Placement::from_scores(scores));
}

// ===== Then Steps =====

#[then(expr = "{string} should be on place {int}")]
async fn on_place(world: &mut SessionWorld, name: String, place: usize) {
    let placement = world.rating.placement.as_ref().expect("No placement");
    assert_eq!(
        placement
            .rank_of(&player(world, &name))
            .map(|rank| rank + 1),
        Some(place),
        "{name} on wrong place"
    );
}

#[then(expr = "{string} should not be placed")]
async fn not_placed(world: &mut SessionWorld, name: String) {
    let placement = world.rating.placement.as_ref().expect("No placement");
    assert_eq!(placement.rank_of(&player(world, &name)), None);
}

#[then(expr = "the placement should be rejected with {string}")]
async fn placement_rejected(world: &mut SessionWorld, message: String) {
    assert!(world.rating.placement.is_none(), "Placement was accepted");
    let error = world.rating.error.as_deref().expect("No error");
    assert!(error.contains(&message), "Unexpected error: {error}");
}
