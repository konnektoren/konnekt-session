use konnekt_session_core::ParticipantId;

/// Errors of the rating domain.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RatingError {
    #[error("Rating key must have a non-empty game identifier")]
    EmptyGame,

    #[error("A placement needs at least two players, got {0}")]
    TooFewPlayers(usize),

    #[error("A placement must not contain empty groups")]
    EmptyGroup,

    #[error("Player {0} appears more than once in the placement")]
    DuplicatePlayer(ParticipantId),

    #[error("No rating given for player {0} of the placement")]
    MissingRating(ParticipantId),

    #[error("Rating given for player {0} who is not in the placement")]
    UnexpectedRating(ParticipantId),

    #[error("Weight must be within 0.0..=1.0, got {0}")]
    InvalidWeight(f64),
}

pub type Result<T> = std::result::Result<T, RatingError>;
