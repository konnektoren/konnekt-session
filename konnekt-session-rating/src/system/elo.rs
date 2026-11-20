use skillratings::elo::{EloConfig, EloRating, elo_rating_period};

use super::{CLIENT_RATING_CAP, RatingSystem, blend, opponents, validate};
use konnekt_session_core::ParticipantId;

use crate::domain::Placement;
use crate::error::Result;

/// Classic Elo: a single number, simple to explain.
#[derive(Debug, Clone, Copy)]
pub struct EloSystem {
    pub config: EloConfig,
    /// Rating of a new player.
    pub initial: EloRating,
    /// Ratings never exceed this value, if set.
    pub cap: Option<f64>,
}

impl Default for EloSystem {
    /// Starts at 1000 with K = 32, capped at [`CLIENT_RATING_CAP`].
    fn default() -> Self {
        Self {
            config: EloConfig::new(),
            initial: EloRating::new(),
            cap: Some(CLIENT_RATING_CAP),
        }
    }
}

impl RatingSystem for EloSystem {
    type Rating = EloRating;

    fn initial(&self) -> EloRating {
        self.initial
    }

    fn value(&self, rating: &EloRating) -> f64 {
        rating.rating
    }

    fn rate(
        &self,
        ratings: &[(ParticipantId, EloRating)],
        placement: &Placement,
        weight: f64,
    ) -> Result<Vec<(ParticipantId, EloRating)>> {
        validate(ratings, placement, weight)?;
        Ok(ratings
            .iter()
            .map(|(player, old)| {
                let results = opponents(player, ratings, placement);
                let new = elo_rating_period(old, &results, &self.config);
                let rating = blend(old.rating, new.rating, weight);
                let rating = self.cap.map_or(rating, |cap| rating.min(cap));
                (*player, EloRating { rating })
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn equal_players_move_by_half_k() {
        let system = EloSystem::default();
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let ratings = [(a, system.initial()), (b, system.initial())];
        let rated = system
            .rate(&ratings, &Placement::win(a, b).unwrap(), 1.0)
            .unwrap();
        assert!((rated[0].1.rating - 1016.0).abs() < 1e-9);
        assert!((rated[1].1.rating - 984.0).abs() < 1e-9);

        let ghost = system
            .rate(&ratings, &Placement::win(a, b).unwrap(), 0.25)
            .unwrap();
        assert!((ghost[0].1.rating - 1004.0).abs() < 1e-9);
    }
}
