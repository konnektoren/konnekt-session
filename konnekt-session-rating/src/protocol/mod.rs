//! Protocols between players, built as sans-IO state machines (ADR-0025).
//!
//! A protocol machine only decides **which state** a match is in. Signing,
//! sending and storing are actions run by the caller after a transition.
//! See `konnekt-session-workspace/protocols/rating-match.md`.

pub mod matchmaking;
pub mod rating_match;

pub use rating_match::{
    DynamicRatingMatch, MatchConfig, RatingMatchEvent, RatingMatchState, RejoinTimedOut,
    RoundSigned,
};
