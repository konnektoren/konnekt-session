use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::error::{RatingError, Result};
use konnekt_session_core::ParticipantId;

/// Outcome of a match as placements: groups of tied players, best first.
///
/// - 1v1 win of `a`: `[[a], [b]]`
/// - draw: `[[a, b]]`
/// - free-for-all: `[[a], [b, c], [d]]`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(try_from = "Vec<Vec<ParticipantId>>")]
pub struct Placement(Vec<Vec<ParticipantId>>);

impl Placement {
    /// Validates: at least two players, no empty group, no player twice.
    pub fn new(groups: Vec<Vec<ParticipantId>>) -> Result<Self> {
        if groups.iter().any(Vec::is_empty) {
            return Err(RatingError::EmptyGroup);
        }
        let mut seen = HashSet::new();
        for player in groups.iter().flatten() {
            if !seen.insert(*player) {
                return Err(RatingError::DuplicatePlayer(*player));
            }
        }
        if seen.len() < 2 {
            return Err(RatingError::TooFewPlayers(seen.len()));
        }
        Ok(Self(groups))
    }

    /// `winner` placed ahead of `loser`.
    pub fn win(winner: ParticipantId, loser: ParticipantId) -> Result<Self> {
        Self::new(vec![vec![winner], vec![loser]])
    }

    /// All players tied.
    pub fn draw(players: Vec<ParticipantId>) -> Result<Self> {
        Self::new(vec![players])
    }

    /// Ranks players by score, higher first; equal scores tie.
    pub fn from_scores(scores: impl IntoIterator<Item = (ParticipantId, u32)>) -> Result<Self> {
        let mut scores: Vec<_> = scores.into_iter().collect();
        scores.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut groups: Vec<Vec<ParticipantId>> = Vec::new();
        let mut last = None;
        for (player, score) in scores {
            match groups.last_mut() {
                Some(group) if last == Some(score) => group.push(player),
                _ => groups.push(vec![player]),
            }
            last = Some(score);
        }
        Self::new(groups)
    }

    pub fn groups(&self) -> &[Vec<ParticipantId>] {
        &self.0
    }

    pub fn players(&self) -> impl Iterator<Item = &ParticipantId> {
        self.0.iter().flatten()
    }

    /// Zero-based rank of a player (tied players share a rank), if present.
    pub fn rank_of(&self, player: &ParticipantId) -> Option<usize> {
        self.0.iter().position(|group| group.contains(player))
    }
}

impl TryFrom<Vec<Vec<ParticipantId>>> for Placement {
    type Error = RatingError;

    fn try_from(groups: Vec<Vec<ParticipantId>>) -> Result<Self> {
        Self::new(groups)
    }
}

impl From<Placement> for Vec<Vec<ParticipantId>> {
    fn from(placement: Placement) -> Self {
        placement.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn players() -> (ParticipantId, ParticipantId) {
        (Uuid::new_v4(), Uuid::new_v4())
    }

    #[test]
    fn win_ranks_winner_first() {
        let (a, b) = players();
        let placement = Placement::win(a, b).unwrap();
        assert_eq!(placement.rank_of(&a), Some(0));
        assert_eq!(placement.rank_of(&b), Some(1));
    }

    #[test]
    fn draw_shares_rank() {
        let (a, b) = players();
        let placement = Placement::draw(vec![a, b]).unwrap();
        assert_eq!(placement.rank_of(&a), placement.rank_of(&b));
        assert_eq!(placement.players().count(), 2);
    }

    #[test]
    fn unknown_player_has_no_rank() {
        let (a, b) = players();
        let placement = Placement::win(a, b).unwrap();
        assert_eq!(placement.rank_of(&Uuid::new_v4()), None);
    }

    #[test]
    fn rejects_invalid_placements() {
        let (a, b) = players();
        assert_eq!(Placement::draw(vec![a]), Err(RatingError::TooFewPlayers(1)));
        assert_eq!(
            Placement::new(vec![vec![a], vec![]]),
            Err(RatingError::EmptyGroup)
        );
        assert_eq!(
            Placement::new(vec![vec![a, b], vec![a]]),
            Err(RatingError::DuplicatePlayer(a))
        );
    }

    #[test]
    fn from_scores_groups_ties() {
        let (a, b) = players();
        let c = Uuid::new_v4();
        let placement = Placement::from_scores([(a, 50), (b, 90), (c, 50)]).unwrap();
        assert_eq!(placement.rank_of(&b), Some(0));
        assert_eq!(placement.rank_of(&a), Some(1));
        assert_eq!(placement.rank_of(&c), Some(1));
        assert_eq!(
            Placement::from_scores([(a, 1)]),
            Err(RatingError::TooFewPlayers(1))
        );
    }

    #[test]
    fn serializes_as_nested_arrays() {
        let (a, b) = players();
        let placement = Placement::win(a, b).unwrap();
        let json = serde_json::to_string(&placement).unwrap();
        assert_eq!(json, serde_json::to_string(&vec![vec![a], vec![b]]).unwrap());
        assert_eq!(serde_json::from_str::<Placement>(&json).unwrap(), placement);
    }

    #[test]
    fn deserialization_validates() {
        let (a, _) = players();
        let json = serde_json::to_string(&vec![vec![a], vec![a]]).unwrap();
        assert!(serde_json::from_str::<Placement>(&json).is_err());
    }
}
