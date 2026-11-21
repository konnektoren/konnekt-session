//! Rating match protocol: two players co-sign every round and the match record.
//!
//! The machine holds only the control state. Counters the guards need
//! (signed rounds) arrive as event payloads, so the caller, which stores the
//! co-signed rounds anyway, stays the single owner of that data.

use state_machines::state_machine;

/// Fixed configuration of one match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchConfig {
    /// Number of rounds in the match (Freestyle Duel: 3).
    pub rounds: u8,
}

/// Payload of `round_signed`: the round that is now co-signed (1-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoundSigned {
    pub round: u8,
}

/// Payload of `rejoin_timeout`: how many rounds are co-signed so far.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RejoinTimedOut {
    pub signed_rounds: u8,
}

state_machine! {
    name: RatingMatch,
    context: MatchConfig,
    dynamic: true,

    initial: ExchangingSnapshots,
    states: [
        ExchangingSnapshots,
        superstate Active {
            state Playing,
            state SigningRound,
            state SigningMatch,
        },
        WaitingForOpponent,
        Rated,
        Forfeited,
        Disagreement,
        Aborted,
    ],
    final_states: [Rated, Forfeited, Disagreement, Aborted],
    events {
        snapshots_verified {
            transition: { from: ExchangingSnapshots, to: Playing }
        }
        snapshot_rejected {
            transition: { from: ExchangingSnapshots, to: Aborted }
        }
        snapshot_timeout {
            transition: { from: ExchangingSnapshots, to: Aborted }
        }
        round_finished {
            transition: { from: Playing, to: SigningRound }
        }
        round_signed {
            payload: RoundSigned,
            branching: true,
            transition: { from: SigningRound, to: Playing, guards: [rounds_left] }
            transition: { from: SigningRound, to: SigningMatch, fallback: true }
        }
        round_mismatch {
            transition: { from: SigningRound, to: Disagreement }
        }
        match_signed {
            transition: { from: SigningMatch, to: Rated }
        }
        match_mismatch {
            transition: { from: SigningMatch, to: Disagreement }
        }
        opponent_left {
            transition: { from: Active, to: WaitingForOpponent }
        }
        signature_timeout {
            transition: { from: SigningRound, to: WaitingForOpponent }
            transition: { from: SigningMatch, to: WaitingForOpponent }
        }
        opponent_rejoined {
            transition: { from: WaitingForOpponent, to: Active, history: deep }
        }
        rejoin_timeout {
            payload: RejoinTimedOut,
            branching: true,
            transition: { from: WaitingForOpponent, to: Forfeited, guards: [has_signed_round] }
            transition: { from: WaitingForOpponent, to: Aborted, fallback: true }
        }
    }
}

impl<S> RatingMatch<S> {
    /// More rounds follow after the one just co-signed.
    fn rounds_left(&self, ctx: &MatchConfig, signed: &RoundSigned) -> bool {
        signed.round < ctx.rounds
    }

    /// A forfeit needs at least one co-signed round as evidence.
    fn has_signed_round(&self, _ctx: &MatchConfig, timed_out: &RejoinTimedOut) -> bool {
        timed_out.signed_rounds > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Event = RatingMatchEvent;
    type State = RatingMatchState;

    fn new_match(rounds: u8) -> DynamicRatingMatch {
        DynamicRatingMatch::new(MatchConfig { rounds })
    }

    fn handle(machine: &mut DynamicRatingMatch, events: impl IntoIterator<Item = Event>) {
        for event in events {
            machine.handle(event).expect("transition");
        }
    }

    fn play_round(machine: &mut DynamicRatingMatch, round: u8) {
        handle(
            machine,
            [
                Event::RoundFinished,
                Event::RoundSigned(RoundSigned { round }),
            ],
        );
    }

    #[test]
    fn starts_exchanging_snapshots() {
        assert_eq!(new_match(3).current_state(), State::ExchangingSnapshots);
    }

    #[test]
    fn happy_path_three_rounds_is_rated() {
        let mut m = new_match(3);
        handle(&mut m, [Event::SnapshotsVerified]);
        assert_eq!(m.current_state(), State::Playing);

        play_round(&mut m, 1);
        assert_eq!(m.current_state(), State::Playing);
        play_round(&mut m, 2);
        assert_eq!(m.current_state(), State::Playing);
        play_round(&mut m, 3);
        assert_eq!(m.current_state(), State::SigningMatch);

        handle(&mut m, [Event::MatchSigned]);
        assert_eq!(m.current_state(), State::Rated);
        assert!(m.is_finished());
    }

    #[test]
    fn single_round_match_goes_straight_to_match_signing() {
        let mut m = new_match(1);
        handle(&mut m, [Event::SnapshotsVerified]);
        play_round(&mut m, 1);
        assert_eq!(m.current_state(), State::SigningMatch);
    }

    #[test]
    fn snapshot_problems_abort() {
        for event in [Event::SnapshotRejected, Event::SnapshotTimeout] {
            let mut m = new_match(3);
            handle(&mut m, [event]);
            assert_eq!(m.current_state(), State::Aborted);
        }
    }

    #[test]
    fn mismatching_hashes_end_in_disagreement() {
        let mut m = new_match(3);
        handle(
            &mut m,
            [
                Event::SnapshotsVerified,
                Event::RoundFinished,
                Event::RoundMismatch,
            ],
        );
        assert_eq!(m.current_state(), State::Disagreement);

        let mut m = new_match(1);
        handle(&mut m, [Event::SnapshotsVerified]);
        play_round(&mut m, 1);
        handle(&mut m, [Event::MatchMismatch]);
        assert_eq!(m.current_state(), State::Disagreement);
    }

    #[test]
    fn rejoin_returns_to_the_state_that_was_left() {
        // Left while playing
        let mut m = new_match(3);
        handle(&mut m, [Event::SnapshotsVerified, Event::OpponentLeft]);
        assert_eq!(m.current_state(), State::WaitingForOpponent);
        handle(&mut m, [Event::OpponentRejoined]);
        assert_eq!(m.current_state(), State::Playing);

        // Left while signing a round
        handle(&mut m, [Event::RoundFinished, Event::OpponentLeft]);
        handle(&mut m, [Event::OpponentRejoined]);
        assert_eq!(m.current_state(), State::SigningRound);

        // Stopped signing the match
        let mut m = new_match(1);
        handle(&mut m, [Event::SnapshotsVerified]);
        play_round(&mut m, 1);
        handle(&mut m, [Event::SignatureTimeout, Event::OpponentRejoined]);
        assert_eq!(m.current_state(), State::SigningMatch);
    }

    #[test]
    fn rejoin_timeout_forfeits_only_with_a_signed_round() {
        let mut m = new_match(3);
        handle(&mut m, [Event::SnapshotsVerified]);
        play_round(&mut m, 1);
        handle(
            &mut m,
            [
                Event::OpponentLeft,
                Event::RejoinTimeout(RejoinTimedOut { signed_rounds: 1 }),
            ],
        );
        assert_eq!(m.current_state(), State::Forfeited);

        let mut m = new_match(3);
        handle(
            &mut m,
            [
                Event::SnapshotsVerified,
                Event::OpponentLeft,
                Event::RejoinTimeout(RejoinTimedOut { signed_rounds: 0 }),
            ],
        );
        assert_eq!(m.current_state(), State::Aborted);
    }

    #[test]
    fn invalid_events_are_rejected_without_changing_state() {
        let mut m = new_match(3);
        assert!(m.handle(Event::MatchSigned).is_err());
        assert!(m.handle(Event::RoundFinished).is_err());
        assert_eq!(m.current_state(), State::ExchangingSnapshots);

        handle(&mut m, [Event::SnapshotsVerified]);
        assert!(
            m.handle(Event::RoundSigned(RoundSigned { round: 1 }))
                .is_err()
        );
        assert!(m.handle(Event::OpponentRejoined).is_err());
        assert_eq!(m.current_state(), State::Playing);
    }

    #[test]
    fn final_states_accept_no_events() {
        let mut m = new_match(1);
        handle(&mut m, [Event::SnapshotRejected]);
        assert!(m.handle(Event::SnapshotsVerified).is_err());
        assert!(m.handle(Event::OpponentLeft).is_err());
        assert_eq!(m.current_state(), State::Aborted);
    }

    #[test]
    fn schema_is_valid() {
        let schema = RatingMatch::<Playing>::schema();
        let diagnostics = schema.validate();
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn mermaid_diagram_lists_all_states() {
        let mermaid = RatingMatch::<Playing>::schema().to_mermaid();
        println!("{mermaid}");
        for state in [
            "ExchangingSnapshots",
            "Playing",
            "SigningRound",
            "SigningMatch",
            "WaitingForOpponent",
            "Rated",
            "Forfeited",
            "Disagreement",
            "Aborted",
        ] {
            assert!(mermaid.contains(state), "missing {state}");
        }
    }

    /// The protocol doc embeds the generated diagram after a marker line.
    /// Skipped when the workspace docs are not next to the crate (e.g. packaged).
    #[test]
    fn docs_diagram_matches_definition() {
        const DOC: &str = "../konnekt-session-workspace/protocols/rating-match.md";
        const MARKER: &str = "%% generated: RatingMatch";

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(DOC);
        let Ok(doc) = std::fs::read_to_string(&path) else {
            eprintln!("skipped: {} not found", path.display());
            return;
        };
        let after_marker = doc
            .split_once(MARKER)
            .expect("marker missing in rating-match.md")
            .1;
        let block = after_marker
            .split_once("```mermaid\n")
            .and_then(|(_, rest)| rest.split_once("```"))
            .expect("mermaid block after marker")
            .0;

        let generated = RatingMatch::<Playing>::schema().to_mermaid();
        assert_eq!(
            block.trim(),
            generated.trim(),
            "rating-match.md is out of date; paste the output of \
             `cargo test -p konnekt-session-rating mermaid -- --nocapture`"
        );
    }
}
