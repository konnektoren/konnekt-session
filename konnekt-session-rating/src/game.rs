use konnekt_session_core::ActivityRun;

use crate::domain::{Placement, RatingKey};

/// Implemented by a game that wants its matches rated.
///
/// The game stays in the consuming app (usually inside its anti-corruption
/// layer, where core's [`ActivityRun`] is available). It only tells the library
/// which rating a match counts for and who placed how.
///
/// All methods must be **pure and deterministic**: every client computes the
/// same outcome from the same run, so the match record can be co-signed.
pub trait RatedGame {
    /// The rating this game's matches count for.
    fn rating_key(&self) -> RatingKey;

    /// Who placed how in this run; `None` if the run is not rated
    /// (cancelled, incomplete, everyone skipped, ...).
    fn placement(&self, run: &ActivityRun) -> Option<Placement>;

    /// Game-specific proof data, hashed into the match record
    /// (e.g. both sheets and all scores).
    fn evidence(&self, run: &ActivityRun) -> Vec<u8>;

    /// How strongly the match counts, `0.0..=1.0`: `1.0` for a normal match,
    /// less for e.g. a ghost game.
    fn weight(&self) -> f64 {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use konnekt_session_core::{ActivityConfig, RunStatus, domain::ActivityResult};
    use uuid::Uuid;

    use super::*;

    /// Higher score wins, like the duet's points.
    struct HighScore;

    impl RatedGame for HighScore {
        fn rating_key(&self) -> RatingKey {
            RatingKey::new("test.high-score.v1").unwrap()
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

    fn run(players: &[Uuid]) -> ActivityRun {
        let config = ActivityConfig::new("high-score".into(), "test".into(), Default::default());
        ActivityRun::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            config,
            players.iter().copied().collect::<HashSet<_>>(),
        )
    }

    #[test]
    fn rates_a_completed_run() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let mut run = run(&[a, b]);
        run.submit_result(ActivityResult::new(run.id(), a).with_score(80))
            .unwrap();
        assert_eq!(
            HighScore.placement(&run),
            None,
            "incomplete runs are not rated"
        );
        run.submit_result(ActivityResult::new(run.id(), b).with_score(60))
            .unwrap();
        assert_eq!(
            HighScore.placement(&run),
            Some(Placement::win(a, b).unwrap())
        );
        assert_eq!(HighScore.weight(), 1.0);
    }
}
