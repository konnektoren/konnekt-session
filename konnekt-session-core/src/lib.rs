pub mod activities;
pub mod application;
pub mod domain;

// Re-export activity types
pub use activities::{EchoChallenge, EchoResult};

pub use domain::{
    Lobby, LobbyError, LobbyRole, Participant, ParticipantError, ParticipationMode, Timestamp,
};

pub use application::runtime::{CommandQueue, DomainLoop, QueueError};
pub use application::{DomainCommand, DomainEvent, DomainEventLoop};
