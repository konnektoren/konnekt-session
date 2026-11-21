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

impl MatchConfig {
    pub(crate) fn has_rounds_after(&self, round: u8) -> bool {
        round < self.rounds
    }
}

impl RejoinTimedOut {
    pub(crate) fn has_signed_round(&self) -> bool {
        self.signed_rounds > 0
    }
}
