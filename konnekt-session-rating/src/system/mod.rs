//! Rating algorithms, wrapping [`skillratings`].

mod elo;
mod glicko2;

pub use elo::EloSystem;
pub use glicko2::Glicko2System;

use std::collections::HashSet;
use std::fmt::Debug;

use serde::{Serialize, de::DeserializeOwned};
use skillratings::Outcomes;

use konnekt_session_core::ParticipantId;

use crate::domain::Placement;
use crate::error::{RatingError, Result};

/// Highest rating a client-side (unverified) rating can reach.
pub const CLIENT_RATING_CAP: f64 = 1500.0;

/// A rating algorithm.
pub trait RatingSystem {
    type Rating: Clone + Debug + PartialEq + Serialize + DeserializeOwned;

    /// Rating of a new player.
    fn initial(&self) -> Self::Rating;

    /// The number shown to players.
    fn value(&self, rating: &Self::Rating) -> f64;

    /// New ratings after a match, in the order of `ratings`.
    ///
    /// `ratings` must contain exactly the players of `placement`. `weight`
    /// (`0.0..=1.0`) scales the change: `1.0` normal, `0.0` no change.
    fn rate(
        &self,
        ratings: &[(ParticipantId, Self::Rating)],
        placement: &Placement,
        weight: f64,
    ) -> Result<Vec<(ParticipantId, Self::Rating)>>;
}

/// Checks that ratings and placement cover the same players, and the weight.
fn validate<R>(ratings: &[(ParticipantId, R)], placement: &Placement, weight: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&weight) {
        return Err(RatingError::InvalidWeight(weight));
    }
    let rated: HashSet<_> = ratings.iter().map(|(player, _)| *player).collect();
    if let Some(player) = placement.players().find(|player| !rated.contains(player)) {
        return Err(RatingError::MissingRating(*player));
    }
    if let Some((player, _)) = ratings
        .iter()
        .find(|(player, _)| placement.rank_of(player).is_none())
    {
        return Err(RatingError::UnexpectedRating(*player));
    }
    Ok(())
}

/// Every other player with the outcome from `player`'s point of view.
/// A free-for-all is rated as one rating period of pairwise games.
fn opponents<R: Clone>(
    player: &ParticipantId,
    ratings: &[(ParticipantId, R)],
    placement: &Placement,
) -> Vec<(R, Outcomes)> {
    let rank = placement.rank_of(player);
    ratings
        .iter()
        .filter(|(other, _)| other != player)
        .map(|(other, rating)| {
            let outcome = match rank.cmp(&placement.rank_of(other)) {
                std::cmp::Ordering::Less => Outcomes::WIN,
                std::cmp::Ordering::Greater => Outcomes::LOSS,
                std::cmp::Ordering::Equal => Outcomes::DRAW,
            };
            (rating.clone(), outcome)
        })
        .collect()
}

/// `old + weight * (new - old)`
fn blend(old: f64, new: f64, weight: f64) -> f64 {
    old + weight * (new - old)
}
