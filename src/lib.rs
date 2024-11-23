#[cfg(feature = "yew")]
pub mod components;
pub mod config;
pub mod handler;
pub mod model;

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "yew")]
pub mod example;

pub mod prelude {
    #[cfg(feature = "yew")]
    pub use crate::components::*;
    pub use crate::config::Config;
    pub use crate::handler::LocalLobbyCommandHandler;
    pub use crate::model::Activity;
    pub use crate::model::ActivityCatalog;
    pub use crate::model::ActivityResult;
    pub use crate::model::ActivityResultTrait;
    pub use crate::model::ActivityStatus;
    pub use crate::model::ActivityTrait;
    pub use crate::model::CommandError;
    pub use crate::model::Identifiable;
    pub use crate::model::Lobby;
    pub use crate::model::LobbyCommand;
    pub use crate::model::LobbyCommandHandler;
    pub use crate::model::Named;
    pub use crate::model::Player;
    pub use crate::model::PlayerId;
    pub use crate::model::PlayerTrait;
    pub use crate::model::Role;
}
