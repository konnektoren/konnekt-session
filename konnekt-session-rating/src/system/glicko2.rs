use skillratings::glicko2::{Glicko2Config, Glicko2Rating, glicko2_rating_period};

use super::{CLIENT_RATING_CAP, RatingSystem, blend, opponents, validate};
use konnekt_session_core::ParticipantId;

use crate::domain::Placement;
use crate::error::Result;

/// Glicko-2: rating plus deviation (uncertainty) and volatility.
/// Default for Konnekt Session.
#[derive(Debug, Clone, Copy)]
pub struct Glicko2System {
    pub config: Glicko2Config,
    /// Rating of a new player.
    pub initial: Glicko2Rating,
    /// Ratings never exceed this value, if set.
    pub cap: Option<f64>,
}

impl Default for Glicko2System {
    /// Starts at 1000 with the standard deviation and volatility,
    /// capped at [`CLIENT_RATING_CAP`].
    fn default() -> Self {
        Self {
            config: Glicko2Config::new(),
            initial: Glicko2Rating {
                rating: 1000.0,
                ..Glicko2Rating::new()
            },
            cap: Some(CLIENT_RATING_CAP),
        }
    }
}

impl RatingSystem for Glicko2System {
    type Rating = Glicko2Rating;

    fn initial(&self) -> Glicko2Rating {
        self.initial
    }

    fn value(&self, rating: &Glicko2Rating) -> f64 {
        rating.rating
    }

    fn rate(
        &self,
        ratings: &[(ParticipantId, Glicko2Rating)],
        placement: &Placement,
        weight: f64,
    ) -> Result<Vec<(ParticipantId, Glicko2Rating)>> {
        validate(ratings, placement, weight)?;
        Ok(ratings
            .iter()
            .map(|(player, old)| {
                let results = opponents(player, ratings, placement);
                let new = glicko2_rating_period(old, &results, &self.config);
                let rating = blend(old.rating, new.rating, weight);
                let rated = Glicko2Rating {
                    rating: self.cap.map_or(rating, |cap| rating.min(cap)),
                    deviation: blend(old.deviation, new.deviation, weight),
                    volatility: blend(old.volatility, new.volatility, weight),
                };
                (*player, rated)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::error::RatingError;

    fn new_players() -> (ParticipantId, ParticipantId) {
        (Uuid::new_v4(), Uuid::new_v4())
    }

    fn rate(
        system: &Glicko2System,
        a: ParticipantId,
        b: ParticipantId,
        placement: Placement,
        weight: f64,
    ) -> (Glicko2Rating, Glicko2Rating) {
        let ratings = [(a, system.initial()), (b, system.initial())];
        let rated = system.rate(&ratings, &placement, weight).unwrap();
        (rated[0].1, rated[1].1)
    }

    #[test]
    fn starts_at_1000() {
        assert_eq!(
            Glicko2System::default().value(&Glicko2System::default().initial()),
            1000.0
        );
    }

    #[test]
    fn winner_gains_and_loser_loses() {
        let system = Glicko2System::default();
        let (a, b) = new_players();
        let (ra, rb) = rate(&system, a, b, Placement::win(a, b).unwrap(), 1.0);
        assert!(ra.rating > 1000.0);
        assert!(rb.rating < 1000.0);
        assert!(
            (ra.rating - 1000.0 - (1000.0 - rb.rating)).abs() < 1e-6,
            "symmetric"
        );
        assert!(ra.deviation < 350.0, "uncertainty shrinks after a game");
    }

    #[test]
    fn draw_between_equals_keeps_rating() {
        let system = Glicko2System::default();
        let (a, b) = new_players();
        let (ra, rb) = rate(&system, a, b, Placement::draw(vec![a, b]).unwrap(), 1.0);
        assert!((ra.rating - 1000.0).abs() < 1e-6);
        assert!((rb.rating - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn weight_scales_the_change() {
        let system = Glicko2System::default();
        let (a, b) = new_players();
        let win = Placement::win(a, b).unwrap();
        let (full, _) = rate(&system, a, b, win.clone(), 1.0);
        let (ghost, _) = rate(&system, a, b, win.clone(), 0.25);
        let (none, _) = rate(&system, a, b, win, 0.0);
        assert!((ghost.rating - 1000.0 - 0.25 * (full.rating - 1000.0)).abs() < 1e-6);
        assert_eq!(none, system.initial());
    }

    #[test]
    fn rating_is_capped() {
        let system = Glicko2System::default();
        let (a, b) = new_players();
        let top = Glicko2Rating {
            rating: 1499.0,
            ..Glicko2Rating::new()
        };
        let rated = system
            .rate(&[(a, top), (b, top)], &Placement::win(a, b).unwrap(), 1.0)
            .unwrap();
        assert_eq!(rated[0].1.rating, CLIENT_RATING_CAP);

        let uncapped = Glicko2System {
            cap: None,
            ..system
        };
        let rated = uncapped
            .rate(&[(a, top), (b, top)], &Placement::win(a, b).unwrap(), 1.0)
            .unwrap();
        assert!(rated[0].1.rating > CLIENT_RATING_CAP);
    }

    #[test]
    fn free_for_all_orders_ratings() {
        let system = Glicko2System::default();
        let (a, b) = new_players();
        let c = Uuid::new_v4();
        let ratings = [
            (c, system.initial()),
            (a, system.initial()),
            (b, system.initial()),
        ];
        let placement = Placement::new(vec![vec![a], vec![b], vec![c]]).unwrap();
        let rated = system.rate(&ratings, &placement, 1.0).unwrap();
        let value = |player| rated.iter().find(|(p, _)| *p == player).unwrap().1.rating;
        assert!(value(a) > value(b) && value(b) > value(c));
        assert_eq!(rated[0].0, c, "keeps input order");
    }

    #[test]
    fn rejects_mismatched_players_and_weights() {
        let system = Glicko2System::default();
        let (a, b) = new_players();
        let win = Placement::win(a, b).unwrap();
        let initial = system.initial();
        assert_eq!(
            system.rate(&[(a, initial)], &win, 1.0),
            Err(RatingError::MissingRating(b))
        );
        let stranger = Uuid::new_v4();
        assert_eq!(
            system.rate(
                &[(a, initial), (b, initial), (stranger, initial)],
                &win,
                1.0
            ),
            Err(RatingError::UnexpectedRating(stranger))
        );
        assert_eq!(
            system.rate(&[(a, initial), (b, initial)], &win, 1.5),
            Err(RatingError::InvalidWeight(1.5))
        );
        assert!(
            system
                .rate(&[(a, initial), (b, initial)], &win, f64::NAN)
                .is_err()
        );
    }
}
