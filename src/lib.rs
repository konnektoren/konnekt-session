#[cfg(feature = "yew")]
pub mod components;
pub mod model;

#[cfg(feature = "server")]
pub mod server;

pub mod prelude {
    #[cfg(feature = "yew")]
    pub use crate::components::*;
    pub use crate::model::Activity;
    pub use crate::model::ActivityCatalog;
    pub use crate::model::ActivityData;
    pub use crate::model::ActivityStatus;
    pub use crate::model::CommandError;
    pub use crate::model::Identifiable;
    pub use crate::model::Lobby;
    pub use crate::model::LobbyCommand;
    pub use crate::model::LobbyCommandHandler;
    pub use crate::model::LocalLobbyCommandHandler;
    pub use crate::model::Named;
    pub use crate::model::Player;
    pub use crate::model::PlayerData;
    pub use crate::model::Role;
}
