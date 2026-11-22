//! Domain types and rating-window rules for matchmaking.

mod discovery;
mod message;
mod offer;
mod state;

pub use discovery::{Hello, search_window};
pub use message::Message;
pub use offer::Offer;
pub use state::State;
