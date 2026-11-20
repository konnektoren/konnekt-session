//! Game-independent skill rating for Konnekt Session.
//!
//! Games stay in the consuming app. They implement [`RatedGame`] to report
//! **which rating** a match counts for ([`RatingKey`]) and **who placed how**
//! ([`Placement`]); a [`RatingSystem`] (Glicko-2 by default) turns that into
//! new ratings.

pub mod domain;
pub mod error;
pub mod game;
pub mod system;

pub use domain::{Placement, RatingKey};
pub use error::{RatingError, Result};
pub use game::RatedGame;
pub use system::{CLIENT_RATING_CAP, EloSystem, Glicko2System, RatingSystem};

/// Re-exports of the most important types.
pub mod prelude {
    pub use crate::domain::{Placement, RatingKey};
    pub use crate::error::{RatingError, Result};
    pub use crate::game::RatedGame;
    pub use crate::system::{CLIENT_RATING_CAP, EloSystem, Glicko2System, RatingSystem};
    pub use konnekt_session_core::ParticipantId;
    pub use skillratings::{elo::EloRating, glicko2::Glicko2Rating};
}
