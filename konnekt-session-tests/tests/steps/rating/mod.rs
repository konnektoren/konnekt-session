//! Steps for `tests/features/rating`.

mod placement_steps;
mod rated_game_steps;
mod rating_match_steps;
mod rating_system_steps;

use konnekt_session_tests::SessionWorld;
use uuid::Uuid;

/// A rating player by name: rating players first, then lobby participants.
pub(crate) fn player(world: &SessionWorld, name: &str) -> Uuid {
    world
        .rating
        .players
        .get(name)
        .or_else(|| world.participant_ids.get(name))
        .copied()
        .unwrap_or_else(|| panic!("Unknown player '{name}'"))
}

/// Registers a player, reusing an existing lobby participant ID.
pub(crate) fn register(world: &mut SessionWorld, name: &str) -> Uuid {
    let id = world
        .participant_ids
        .get(name)
        .copied()
        .unwrap_or_else(Uuid::new_v4);
    world.rating.players.insert(name.to_string(), id);
    id
}
